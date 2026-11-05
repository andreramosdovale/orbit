//! Publica a captura no LiveKit, em uma conexão própria (`<id>#screen`, ADR 0006).

use super::CaptureTarget;
use super::audio::{self, CHANNELS, SAMPLE_RATE, SAMPLES_PER_CHANNEL};
use super::encoder::{EncodedUnit, EncoderOutput, HardwareEncoder, Nv12Frame};
use super::video::{self, FrameSink, MAX_FRAMERATE, MAX_HEIGHT, MAX_WIDTH, VideoCapture};
use livekit::options::{AudioEncoding, TrackPublishOptions, VideoCodec, VideoEncoding};
use livekit::prelude::*;
use livekit::webrtc::audio_frame::AudioFrame;
use livekit::webrtc::audio_source::native::NativeAudioSource;
use livekit::webrtc::audio_source::{AudioSourceOptions, RtcAudioSource};
use livekit::webrtc::prelude::I420Buffer;
use livekit::webrtc::rtp_sender::VideoEncoderBackend;
use livekit::webrtc::video_frame::{EncodedFrameType, EncodedVideoCodec, EncodedVideoFrame};
use livekit::webrtc::video_source::native::NativeVideoSource;
use livekit::webrtc::video_source::{RtcVideoSource, VideoResolution};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;

const MAX_VIDEO_BITRATE: u64 = 4_000_000;
const AUDIO_BITRATE: u64 = 128_000;
const AUDIO_QUEUE_MS: u32 = 100;

/// Por que a transmissão acabou sem ter sido parada.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum EndReason {
    TargetClosed,
    ConnectionLost,
    Failed,
}

/// Quem codifica o vídeo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum VideoEncoderKind {
    /// A placa de vídeo, em H.264.
    Hardware,
    /// O processador, em VP8.
    Software,
}

#[derive(Debug, thiserror::Error)]
pub enum ShareError {
    #[error("Não foi possível conectar a transmissão ao servidor.")]
    Connect,
    #[error("O servidor recusou a transmissão.")]
    Publish,
    #[error("Essa janela ou tela não está mais disponível.")]
    Target,
}

/// A transmissão em andamento.
pub struct ScreenShare {
    room: Room,
    video: VideoCapture,
    audio: Option<JoinHandle<()>>,
    encoder: Option<Arc<HardwareEncoder>>,
    encoder_kind: VideoEncoderKind,
    stopping: Arc<AtomicBool>,
}

/// Leva o vídeo codificado pela placa ao LiveKit, e os pedidos do LiveKit ao encoder.
struct SourceOutput {
    source: NativeVideoSource,
    on_failed: Box<dyn FnMut() + Send>,
}

impl EncoderOutput for SourceOutput {
    fn on_unit(&mut self, unit: &EncodedUnit) {
        self.source.capture_encoded_frame(&EncodedVideoFrame {
            codec: EncodedVideoCodec::H264,
            payload: &unit.payload,
            timestamp_us: unit.timestamp_us,
            frame_type: if unit.key {
                EncodedFrameType::Key
            } else {
                EncodedFrameType::Delta
            },
            resolution: VideoResolution {
                width: unit.width,
                height: unit.height,
            },
            frame_metadata: None,
        });
    }

    fn take_keyframe_request(&mut self) -> bool {
        self.source.take_keyframe_request()
    }

    fn take_bitrate_request(&mut self) -> Option<u32> {
        self.source
            .take_rate_control_request()
            .map(|request| u32::try_from(request.target_bitrate_bps).unwrap_or(u32::MAX))
    }

    fn on_failed(&mut self) {
        (self.on_failed)();
    }
}

/// Começa a transmitir. `on_ended` é chamado, no máximo uma vez, se ela acabar sozinha.
pub async fn start(
    target: CaptureTarget,
    url: &str,
    token: &str,
    on_ended: impl Fn(EndReason) + Send + Sync + 'static,
) -> Result<ScreenShare, ShareError> {
    let mut options = RoomOptions::default();
    options.auto_subscribe = false;
    // O erro do SDK pode citar o endereço com o token: fica de fora da mensagem.
    let (room, mut events) = Room::connect(url, token, options)
        .await
        .map_err(|_| ShareError::Connect)?;

    let stopping = Arc::new(AtomicBool::new(false));
    let ended = Arc::new(AtomicBool::new(false));
    let on_ended = Arc::new(on_ended);
    let finish = {
        let (stopping, ended, on_ended) = (stopping.clone(), ended.clone(), on_ended.clone());
        Arc::new(move |reason: EndReason| {
            if !stopping.load(Ordering::SeqCst) && !ended.swap(true, Ordering::SeqCst) {
                on_ended(reason);
            }
        })
    };

    // O encoder abre antes de publicar: a escolha define o codec da publicação.
    let (width, height) = video::target_size(&target).map_err(|_| ShareError::Target)?;
    let encoded_source = NativeVideoSource::new_encoded(VideoResolution { width, height });
    let encoder_finish = finish.clone();
    let output = SourceOutput {
        source: encoded_source.clone(),
        on_failed: Box::new(move || encoder_finish(EndReason::Failed)),
    };
    let encoder =
        tokio::task::spawn_blocking(move || HardwareEncoder::start(width, height, output))
            .await
            .ok()
            .and_then(Result::ok)
            .map(Arc::new);
    if let Some(encoder) = &encoder {
        eprintln!("transmissão pela placa de vídeo: {}", encoder.name());
    }
    let encoder_kind = if encoder.is_some() {
        VideoEncoderKind::Hardware
    } else {
        VideoEncoderKind::Software
    };

    let video_source = if encoder.is_some() {
        encoded_source
    } else {
        NativeVideoSource::new(
            VideoResolution {
                width: MAX_WIDTH,
                height: MAX_HEIGHT,
            },
            true,
        )
    };
    let video_track =
        LocalVideoTrack::create_video_track("screen", RtcVideoSource::Native(video_source.clone()));
    let mut video_options = TrackPublishOptions {
        source: TrackSource::Screenshare,
        video_codec: VideoCodec::VP8,
        video_encoding: Some(VideoEncoding {
            max_bitrate: MAX_VIDEO_BITRATE,
            max_framerate: f64::from(MAX_FRAMERATE),
        }),
        simulcast: false,
        ..TrackPublishOptions::default()
    };
    if encoder.is_some() {
        video_options.video_codec = VideoCodec::H264;
        video_options.video_encoder = VideoEncoderBackend::PreEncoded;
    }
    let published = room
        .local_participant()
        .publish_track(LocalTrack::Video(video_track), video_options)
        .await;
    if published.is_err() {
        let _ = room.close().await;
        return Err(ShareError::Publish);
    }

    // Só uma janela tem áudio: em uma tela não há um aplicativo para isolar.
    let mut audio_thread = None;
    if let CaptureTarget::Window { process_id, .. } = &target {
        let audio_source = NativeAudioSource::new(
            AudioSourceOptions::default(),
            SAMPLE_RATE,
            CHANNELS,
            AUDIO_QUEUE_MS,
        );
        let audio_track = LocalAudioTrack::create_audio_track(
            "screen-audio",
            RtcAudioSource::Native(audio_source.clone()),
        );
        let audio_options = TrackPublishOptions {
            source: TrackSource::ScreenshareAudio,
            dtx: false,
            audio_encoding: Some(AudioEncoding {
                max_bitrate: AUDIO_BITRATE,
            }),
            ..TrackPublishOptions::default()
        };
        let published = room
            .local_participant()
            .publish_track(LocalTrack::Audio(audio_track), audio_options)
            .await;
        if published.is_err() {
            let _ = room.close().await;
            return Err(ShareError::Publish);
        }

        let process_id = *process_id;
        let stop = stopping.clone();
        let runtime = tokio::runtime::Handle::current();
        audio_thread = Some(std::thread::spawn(move || {
            let result = audio::capture_process(process_id, &stop, |samples| {
                let frame = AudioFrame {
                    data: samples.into(),
                    sample_rate: SAMPLE_RATE,
                    num_channels: CHANNELS,
                    samples_per_channel: SAMPLES_PER_CHANNEL,
                };
                let _ = runtime.block_on(audio_source.capture_frame(&frame));
            });
            // Sem áudio, a imagem continua: é melhor do que derrubar a transmissão.
            if let Err(err) = result {
                eprintln!("aviso: o áudio da transmissão parou: {err}");
            }
        }));
    }

    let video_finish = finish.clone();
    let on_video_ended: video::OnEnded = Box::new(move |error| {
        video_finish(if error.is_some() {
            EndReason::Failed
        } else {
            EndReason::TargetClosed
        });
    });
    let sink = match &encoder {
        Some(encoder) => {
            let encoder = encoder.clone();
            FrameSink {
                on_frame: Box::new(move |buffer: &I420Buffer| {
                    encoder.push(Nv12Frame::from_i420(buffer));
                }),
                on_ended: Some(on_video_ended),
            }
        }
        None => FrameSink::to_source(video_source, on_video_ended),
    };
    let video = match video::start(&target, sink) {
        Ok(video) => video,
        Err(_) => {
            stopping.store(true, Ordering::SeqCst);
            let _ = room.close().await;
            if let Some(thread) = audio_thread {
                let _ = thread.join();
            }
            return Err(ShareError::Target);
        }
    };

    tokio::spawn(async move {
        while let Some(event) = events.recv().await {
            if matches!(event, RoomEvent::Disconnected { .. }) {
                finish(EndReason::ConnectionLost);
                break;
            }
        }
    });

    Ok(ScreenShare {
        room,
        video,
        audio: audio_thread,
        encoder,
        encoder_kind,
        stopping,
    })
}

impl ScreenShare {
    pub fn encoder_kind(&self) -> VideoEncoderKind {
        self.encoder_kind
    }

    /// Para a captura e sai da sala.
    pub async fn stop(self) {
        self.stopping.store(true, Ordering::SeqCst);
        // Parar a captura espera a thread dela, que não pode segurar o runtime.
        let video = self.video;
        let audio = self.audio;
        let encoder = self.encoder;
        let _ = tokio::task::spawn_blocking(move || {
            video.stop();
            if let Some(encoder) = encoder {
                encoder.stop();
            }
            if let Some(thread) = audio {
                let _ = thread.join();
            }
        })
        .await;
        let _ = self.room.close().await;
    }
}
