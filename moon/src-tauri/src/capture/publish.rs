//! Publica a captura no LiveKit, em uma conexão própria (`<id>#screen`, ADR 0006).

use super::CaptureTarget;
use super::audio::{self, CHANNELS, SAMPLE_RATE, SAMPLES_PER_CHANNEL};
use super::video::{self, FrameSink, MAX_FRAMERATE, MAX_HEIGHT, MAX_WIDTH, VideoCapture};
use livekit::options::{AudioEncoding, TrackPublishOptions, VideoCodec, VideoEncoding};
use livekit::prelude::*;
use livekit::webrtc::audio_frame::AudioFrame;
use livekit::webrtc::audio_source::native::NativeAudioSource;
use livekit::webrtc::audio_source::{AudioSourceOptions, RtcAudioSource};
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
    stopping: Arc<AtomicBool>,
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

    let video_source = NativeVideoSource::new(
        VideoResolution {
            width: MAX_WIDTH,
            height: MAX_HEIGHT,
        },
        true,
    );
    let video_track =
        LocalVideoTrack::create_video_track("screen", RtcVideoSource::Native(video_source.clone()));
    let video_options = TrackPublishOptions {
        source: TrackSource::Screenshare,
        video_codec: VideoCodec::VP8,
        video_encoding: Some(VideoEncoding {
            max_bitrate: MAX_VIDEO_BITRATE,
            max_framerate: f64::from(MAX_FRAMERATE),
        }),
        simulcast: false,
        ..TrackPublishOptions::default()
    };
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
    let sink = FrameSink::to_source(
        video_source,
        Box::new(move |error| {
            video_finish(if error.is_some() {
                EndReason::Failed
            } else {
                EndReason::TargetClosed
            });
        }),
    );
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
        stopping,
    })
}

impl ScreenShare {
    /// Para a captura e sai da sala.
    pub async fn stop(self) {
        self.stopping.store(true, Ordering::SeqCst);
        // Parar a captura espera a thread dela, que não pode segurar o runtime.
        let video = self.video;
        let audio = self.audio;
        let _ = tokio::task::spawn_blocking(move || {
            video.stop();
            if let Some(thread) = audio {
                let _ = thread.join();
            }
        })
        .await;
        let _ = self.room.close().await;
    }
}
