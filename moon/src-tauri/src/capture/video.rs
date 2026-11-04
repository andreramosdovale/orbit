//! Captura de janela ou tela por Windows Graphics Capture.

use super::CaptureTarget;
use livekit::webrtc::native::yuv_helper::argb_to_i420;
use livekit::webrtc::prelude::{I420Buffer, VideoFrame, VideoRotation};
use livekit::webrtc::video_source::native::NativeVideoSource;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use windows_capture::capture::{CaptureControl, Context, GraphicsCaptureApiHandler};
use windows_capture::frame::Frame;
use windows_capture::graphics_capture_api::InternalCaptureControl;
use windows_capture::monitor::Monitor;
use windows_capture::settings::{
    ColorFormat, CursorCaptureSettings, DirtyRegionSettings, DrawBorderSettings,
    MinimumUpdateIntervalSettings, SecondaryWindowSettings, Settings,
};
use windows_capture::window::Window;

pub const MAX_WIDTH: u32 = 1920;
pub const MAX_HEIGHT: u32 = 1080;
pub const MAX_FRAMERATE: u32 = 30;

/// Cabe o tamanho em 1080p, mantendo a proporção. O resultado é sempre par e nunca aumenta.
pub fn fit_size(width: u32, height: u32) -> (u32, u32) {
    let scale = f64::min(
        1.0,
        f64::min(
            f64::from(MAX_WIDTH) / f64::from(width.max(1)),
            f64::from(MAX_HEIGHT) / f64::from(height.max(1)),
        ),
    );
    let even = |value: f64| ((value as u32) & !1).max(2);
    (
        even(f64::from(width) * scale),
        even(f64::from(height) * scale),
    )
}

#[derive(Debug, thiserror::Error)]
pub enum VideoError {
    #[error("essa janela ou tela não está mais disponível")]
    TargetGone,
    #[error("não foi possível ler um quadro: {0}")]
    Frame(String),
    #[error("não foi possível iniciar a captura: {0}")]
    Start(String),
}

/// Chamado uma vez, quando a captura termina sem ter sido parada.
pub type OnEnded = Box<dyn FnOnce(Option<String>) + Send>;

/// Recebe os quadros. Fica separado de quem publica: os testes usam o `on_frame` deles.
pub struct FrameSink {
    pub on_frame: Box<dyn FnMut(&I420Buffer) + Send>,
    pub on_ended: Option<OnEnded>,
}

impl FrameSink {
    /// Manda para o LiveKit, carimbando o tempo.
    pub fn to_source(source: NativeVideoSource, on_ended: OnEnded) -> Self {
        let start = Instant::now();
        Self {
            on_frame: Box::new(move |buffer| {
                source.capture_frame(&VideoFrame {
                    rotation: VideoRotation::VideoRotation0,
                    timestamp_us: start.elapsed().as_micros() as i64,
                    frame_metadata: None,
                    buffer,
                });
            }),
            on_ended: Some(on_ended),
        }
    }
}

type Flags = Arc<Mutex<Option<FrameSink>>>;

struct Handler {
    sink: FrameSink,
}

impl Handler {
    fn finish(&mut self, error: Option<String>) {
        if let Some(on_ended) = self.sink.on_ended.take() {
            on_ended(error);
        }
    }

    fn convert(frame: &mut Frame) -> Result<I420Buffer, VideoError> {
        let mut pixels = frame
            .buffer()
            .map_err(|err| VideoError::Frame(err.to_string()))?;
        let (width, height, stride) = (pixels.width(), pixels.height(), pixels.row_pitch());
        let mut buffer = I420Buffer::new(width, height);
        let (stride_y, stride_u, stride_v) = buffer.strides();
        let (y, u, v) = buffer.data_mut();
        argb_to_i420(
            pixels.as_raw_buffer(),
            stride,
            y,
            stride_y,
            u,
            stride_u,
            v,
            stride_v,
            width as i32,
            height as i32,
        );
        let (fit_width, fit_height) = fit_size(width, height);
        Ok(if (fit_width, fit_height) == (width, height) {
            buffer
        } else {
            buffer.scale(fit_width as i32, fit_height as i32)
        })
    }
}

impl GraphicsCaptureApiHandler for Handler {
    type Flags = Flags;
    type Error = VideoError;

    fn new(ctx: Context<Self::Flags>) -> Result<Self, Self::Error> {
        let sink = ctx
            .flags
            .lock()
            .ok()
            .and_then(|mut sink| sink.take())
            .ok_or_else(|| VideoError::Start("a captura já tinha sido iniciada".into()))?;
        Ok(Self { sink })
    }

    fn on_frame_arrived(
        &mut self,
        frame: &mut Frame,
        _capture_control: InternalCaptureControl,
    ) -> Result<(), Self::Error> {
        match Self::convert(frame) {
            Ok(buffer) => {
                (self.sink.on_frame)(&buffer);
                Ok(())
            }
            Err(err) => {
                self.finish(Some(err.to_string()));
                Err(err)
            }
        }
    }

    fn on_closed(&mut self) -> Result<(), Self::Error> {
        self.finish(None);
        Ok(())
    }
}

/// A captura em andamento.
pub struct VideoCapture {
    control: CaptureControl<Handler, VideoError>,
}

impl VideoCapture {
    pub fn stop(self) {
        let _ = self.control.stop();
    }
}

fn start_with<T>(
    item: T,
    sink: Flags,
    border: DrawBorderSettings,
) -> Result<VideoCapture, VideoError>
where
    T: TryInto<windows_capture::settings::GraphicsCaptureItemType> + Send + 'static,
{
    let settings = Settings::new(
        item,
        CursorCaptureSettings::WithCursor,
        border,
        SecondaryWindowSettings::Default,
        MinimumUpdateIntervalSettings::Custom(Duration::from_millis(u64::from(
            1000 / MAX_FRAMERATE,
        ))),
        DirtyRegionSettings::Default,
        ColorFormat::Bgra8,
        sink,
    );
    Handler::start_free_threaded(settings)
        .map(|control| VideoCapture { control })
        .map_err(|err| VideoError::Start(err.to_string()))
}

/// Começa a capturar. Sem a borda amarela quando o Windows permite.
pub fn start(target: &CaptureTarget, sink: FrameSink) -> Result<VideoCapture, VideoError> {
    let flags: Flags = Arc::new(Mutex::new(Some(sink)));
    let border = |flags: &Flags, border| match target {
        CaptureTarget::Window { id, .. } => {
            let hwnd: usize = id.parse().map_err(|_| VideoError::TargetGone)?;
            let window = Window::from_raw_hwnd(hwnd as *mut std::ffi::c_void);
            if !window.is_valid() {
                return Err(VideoError::TargetGone);
            }
            start_with(window, flags.clone(), border)
        }
        CaptureTarget::Screen { index, .. } => {
            let monitor =
                Monitor::from_index(*index as usize).map_err(|_| VideoError::TargetGone)?;
            start_with(monitor, flags.clone(), border)
        }
    };
    match border(&flags, DrawBorderSettings::WithoutBorder) {
        Err(VideoError::Start(_)) if flags.lock().is_ok_and(|sink| sink.is_some()) => {
            border(&flags, DrawBorderSettings::Default)
        }
        result => result,
    }
}
