//! H.264 na placa de vídeo, por Media Foundation (spec 014, ADR 0018). É o único trecho da
//! captura com `unsafe`, e os objetos COM nunca saem da thread do encoder.

use livekit::webrtc::native::yuv_helper::i420_to_nv12;
use livekit::webrtc::prelude::{I420Buffer, VideoBuffer};
use std::mem::ManuallyDrop;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use windows::Win32::Foundation::{E_FAIL, HMODULE};
use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_UNKNOWN;
use windows::Win32::Graphics::Direct3D11::{
    D3D11_BIND_SHADER_RESOURCE, D3D11_CREATE_DEVICE_BGRA_SUPPORT,
    D3D11_CREATE_DEVICE_VIDEO_SUPPORT, D3D11_SDK_VERSION, D3D11_TEXTURE2D_DESC,
    D3D11_USAGE_DEFAULT, D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D,
};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_NV12, DXGI_SAMPLE_DESC};
use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, IDXGIAdapter, IDXGIFactory1};
use windows::Win32::Media::MediaFoundation::*;
use windows::Win32::System::Com::{
    COINIT_MULTITHREADED, CoInitializeEx, CoTaskMemFree, CoUninitialize,
};
use windows::Win32::System::Variant::VARIANT;
use windows::core::{GUID, Interface, PWSTR};

pub const MAX_BITRATE: u32 = 4_000_000;
/// Abaixo disso a imagem some: o limite vale mesmo se o LiveKit pedir menos.
const MIN_BITRATE: u32 = 300_000;
const FRAMERATE: u32 = 30;
const GOP_FRAMES: u32 = 120;
/// Quanto esperar pelo quadro de teste de cada encoder.
const SELF_TEST: Duration = Duration::from_secs(3);
/// Repete o último quadro em uma imagem parada, para quem chegar depois receber algo.
const KEEP_ALIVE: Duration = Duration::from_secs(1);
const IDLE: Duration = Duration::from_millis(1);
/// Variável que simula um PC sem encoder em hardware, só nos builds de desenvolvimento.
const NO_HARDWARE_ENV: &str = "ORBIT_TEST_NO_HARDWARE_ENCODER";

#[derive(Debug, thiserror::Error)]
pub enum EncoderError {
    #[error("nenhum encoder em hardware funcionou")]
    Unavailable,
    #[error("erro no encoder: {0}")]
    Windows(#[from] windows::core::Error),
}

/// Um quadro em NV12.
pub struct Nv12Frame {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

impl Nv12Frame {
    /// Converte um quadro da captura.
    pub fn from_i420(buffer: &I420Buffer) -> Self {
        let (width, height) = (buffer.width(), buffer.height());
        let luma = (width * height) as usize;
        let mut data = vec![0u8; luma + luma / 2];
        let (stride_y, stride_u, stride_v) = buffer.strides();
        let (src_y, src_u, src_v) = buffer.data();
        let (dst_y, dst_uv) = data.split_at_mut(luma);
        i420_to_nv12(
            src_y,
            stride_y,
            src_u,
            stride_u,
            src_v,
            stride_v,
            dst_y,
            width,
            dst_uv,
            width,
            width as i32,
            height as i32,
        );
        Self {
            data,
            width,
            height,
        }
    }
}

/// Um quadro já codificado, em Annex-B.
pub struct EncodedUnit {
    pub payload: Vec<u8>,
    pub key: bool,
    pub width: u32,
    pub height: u32,
    pub timestamp_us: i64,
}

/// Quem recebe o vídeo e faz os pedidos de quem assiste.
pub trait EncoderOutput: Send + 'static {
    fn on_unit(&mut self, unit: &EncodedUnit);
    /// Alguém precisa de um quadro-chave.
    fn take_keyframe_request(&mut self) -> bool;
    /// A taxa pedida, em bits por segundo.
    fn take_bitrate_request(&mut self) -> Option<u32>;
    /// O encoder parou de funcionar depois de iniciar.
    fn on_failed(&mut self);
}

/// A taxa aceita para um pedido do LiveKit.
pub fn clamp_bitrate(requested: u32) -> u32 {
    requested.clamp(MIN_BITRATE, MAX_BITRATE)
}

/// A NVIDIA vem antes porque foi a testada, e a placa dedicada, antes da integrada.
pub fn vendor_rank(vendor_id: Option<u32>) -> u8 {
    match vendor_id {
        Some(0x10DE) => 0,
        Some(0x1002) => 1,
        Some(0x8086) => 2,
        _ => 3,
    }
}

/// Lê `VEN_10DE` como `0x10DE`.
pub fn parse_vendor_id(text: &str) -> Option<u32> {
    u32::from_str_radix(text.strip_prefix("VEN_")?, 16).ok()
}

struct Shared {
    latest: Mutex<Option<Nv12Frame>>,
    stop: AtomicBool,
}

/// O encoder em andamento, em uma thread própria.
pub struct HardwareEncoder {
    shared: Arc<Shared>,
    thread: Mutex<Option<JoinHandle<()>>>,
    name: String,
}

impl HardwareEncoder {
    /// Abre o primeiro encoder que passar no quadro de teste. Bloqueia até saber.
    pub fn start(
        width: u32,
        height: u32,
        output: impl EncoderOutput,
    ) -> Result<Self, EncoderError> {
        if cfg!(debug_assertions) && std::env::var_os(NO_HARDWARE_ENV).is_some() {
            return Err(EncoderError::Unavailable);
        }
        let shared = Arc::new(Shared {
            latest: Mutex::new(None),
            stop: AtomicBool::new(false),
        });
        let (ready_tx, ready_rx) = mpsc::channel();
        let thread_shared = shared.clone();
        let thread = std::thread::spawn(move || {
            run_thread(width, height, output, &thread_shared, ready_tx);
        });
        match ready_rx.recv() {
            Ok(Some(name)) => Ok(Self {
                shared,
                thread: Mutex::new(Some(thread)),
                name,
            }),
            _ => {
                let _ = thread.join();
                Err(EncoderError::Unavailable)
            }
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// Entrega um quadro. Se o encoder estiver atrasado, fica só o mais novo.
    pub fn push(&self, frame: Nv12Frame) {
        if let Ok(mut latest) = self.shared.latest.lock() {
            *latest = Some(frame);
        }
    }

    /// Para o encoder e espera a thread dele terminar.
    pub fn stop(&self) {
        self.shared.stop.store(true, Ordering::SeqCst);
        let thread = self.thread.lock().ok().and_then(|mut thread| thread.take());
        if let Some(thread) = thread {
            let _ = thread.join();
        }
    }
}

impl Drop for HardwareEncoder {
    fn drop(&mut self) {
        self.stop();
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum InputMode {
    /// Quadros na memória do sistema: basta para a NVIDIA.
    System,
    /// Com um dispositivo D3D11 associado, ainda com quadros na memória do sistema.
    D3dSystem,
    /// Com quadros em texturas: alguns encoders só aceitam assim.
    D3dTexture,
}

const INPUT_MODES: [InputMode; 3] = [
    InputMode::System,
    InputMode::D3dSystem,
    InputMode::D3dTexture,
];

fn run_thread(
    width: u32,
    height: u32,
    mut output: impl EncoderOutput,
    shared: &Shared,
    ready: mpsc::Sender<Option<String>>,
) {
    // SAFETY: inicia COM e Media Foundation nesta thread e encerra os dois no fim dela, depois
    // que `run_encoder` já liberou todos os objetos.
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        if MFStartup(MF_VERSION, MFSTARTUP_FULL).is_ok() {
            run_encoder(width, height, &mut output, shared, &ready);
            let _ = MFShutdown();
        } else {
            let _ = ready.send(None);
        }
        CoUninitialize();
    }
}

fn run_encoder(
    width: u32,
    height: u32,
    output: &mut impl EncoderOutput,
    shared: &Shared,
    ready: &mpsc::Sender<Option<String>>,
) {
    let mut chosen = None;
    'search: for candidate in enumerate_encoders() {
        for mode in INPUT_MODES {
            let opened = Session::open(&candidate, width, height, mode)
                .and_then(|mut session| session.self_test().map(|()| session));
            match opened {
                Ok(session) => {
                    chosen = Some((candidate, mode, session));
                    break 'search;
                }
                Err(err) => {
                    eprintln!("aviso: {} ({mode:?}) não funcionou: {err}", candidate.name);
                }
            }
        }
    }
    let Some((candidate, mode, session)) = chosen else {
        let _ = ready.send(None);
        return;
    };
    let _ = ready.send(Some(candidate.name.clone()));

    if let Err(err) = encode_loop(session, &candidate, mode, output, shared) {
        eprintln!("aviso: o encoder em hardware parou: {err}");
        output.on_failed();
    }
}

fn encode_loop(
    mut session: Session,
    candidate: &Candidate,
    mode: InputMode,
    output: &mut impl EncoderOutput,
    shared: &Shared,
) -> Result<(), EncoderError> {
    let start = Instant::now();
    let mut last: Option<Nv12Frame> = None;
    let mut last_input = Instant::now();
    let mut has_fresh_frame = false;
    let mut force_key = true;

    while !shared.stop.load(Ordering::SeqCst) {
        force_key |= output.take_keyframe_request();
        if let Some(bitrate) = output.take_bitrate_request() {
            session.set_bitrate(clamp_bitrate(bitrate));
        }

        let event = session.next_event()?;
        if event == Some(METransformHaveOutput)
            && let Some(unit) = session.take_output()?
        {
            output.on_unit(&unit);
        }
        let mut worked = event.is_some();

        if session.needs > 0
            && let Some(frame) = shared
                .latest
                .lock()
                .ok()
                .and_then(|mut latest| latest.take())
        {
            // O tamanho do encoder é fixo: uma janela redimensionada pede outro.
            if (frame.width, frame.height) != (session.width, session.height) {
                drop(session);
                session = Session::open(candidate, frame.width, frame.height, mode)?;
                force_key = true;
            }
            last = Some(frame);
            has_fresh_frame = true;
        }
        // Em uma imagem parada, repete o último quadro: quem chega depois precisa de um.
        let due = has_fresh_frame || force_key || last_input.elapsed() >= KEEP_ALIVE;
        if due
            && session.needs > 0
            && let Some(frame) = &last
        {
            if force_key {
                session.force_keyframe();
                force_key = false;
            }
            session.feed(&frame.data, start.elapsed().as_micros() as i64)?;
            has_fresh_frame = false;
            last_input = Instant::now();
            worked = true;
        }
        if !worked {
            std::thread::sleep(IDLE);
        }
    }
    Ok(())
}

struct Candidate {
    name: String,
    vendor_id: Option<u32>,
    activate: IMFActivate,
}

fn enumerate_encoders() -> Vec<Candidate> {
    let output = MFT_REGISTER_TYPE_INFO {
        guidMajorType: MFMediaType_Video,
        guidSubtype: MFVideoFormat_H264,
    };
    let mut list: *mut Option<IMFActivate> = std::ptr::null_mut();
    let mut count = 0u32;
    let mut found = Vec::new();
    // SAFETY: `list` recebe `count` ponteiros alocados pelo COM. Cada item é retirado uma vez, e o
    // vetor é liberado depois de lido. Só leituras de atributos acontecem nos objetos.
    unsafe {
        if MFTEnumEx(
            MFT_CATEGORY_VIDEO_ENCODER,
            MFT_ENUM_FLAG_HARDWARE | MFT_ENUM_FLAG_SORTANDFILTER,
            None,
            Some(&output),
            &mut list,
            &mut count,
        )
        .is_err()
        {
            return found;
        }
        for index in 0..count as usize {
            if let Some(activate) = (*list.add(index)).take() {
                let name = string_attribute(&activate, &MFT_FRIENDLY_NAME_Attribute)
                    .unwrap_or_else(|| "encoder sem nome".to_owned());
                let vendor_id = string_attribute(&activate, &MFT_ENUM_HARDWARE_VENDOR_ID_Attribute)
                    .as_deref()
                    .and_then(parse_vendor_id);
                found.push(Candidate {
                    name,
                    vendor_id,
                    activate,
                });
            }
        }
        CoTaskMemFree(Some(list as *const _));
    }
    found.sort_by_key(|candidate| vendor_rank(candidate.vendor_id));
    found
}

/// # Safety
/// `activate` precisa ser um objeto válido.
unsafe fn string_attribute(activate: &IMFActivate, key: &GUID) -> Option<String> {
    let mut text = PWSTR::null();
    let mut length = 0u32;
    // SAFETY: a string é alocada pelo COM, copiada e liberada aqui.
    unsafe {
        activate
            .GetAllocatedString(key, &mut text, &mut length)
            .ok()?;
        let copy = text.to_string().ok();
        CoTaskMemFree(Some(text.0 as *const _));
        copy
    }
}

fn pack(high: u32, low: u32) -> u64 {
    (u64::from(high) << 32) | u64::from(low)
}

struct D3d {
    device: ID3D11Device,
    context: ID3D11DeviceContext,
    // Precisa viver tanto quanto o encoder.
    _manager: IMFDXGIDeviceManager,
}

struct Session {
    activate: IMFActivate,
    transform: IMFTransform,
    events: IMFMediaEventGenerator,
    codec: Option<ICodecAPI>,
    provides_samples: bool,
    output_size: u32,
    d3d: Option<D3d>,
    textures: bool,
    width: u32,
    height: u32,
    needs: u32,
}

impl Session {
    fn open(
        candidate: &Candidate,
        width: u32,
        height: u32,
        mode: InputMode,
    ) -> Result<Self, EncoderError> {
        // SAFETY: todos os objetos são criados e usados nesta thread, com COM iniciado por
        // `run_thread`, e os ponteiros passados só valem durante cada chamada.
        unsafe {
            let transform: IMFTransform = candidate.activate.ActivateObject()?;
            let attributes = transform.GetAttributes()?;
            attributes.SetUINT32(&MF_TRANSFORM_ASYNC_UNLOCK, 1)?;
            let _ = attributes.SetUINT32(&MF_LOW_LATENCY, 1);
            let events: IMFMediaEventGenerator = transform.cast()?;

            let d3d = if mode == InputMode::System {
                None
            } else {
                Some(open_d3d(candidate, &transform)?)
            };

            let output = video_type(&MFVideoFormat_H264, width, height)?;
            output.SetUINT32(&MF_MT_AVG_BITRATE, MAX_BITRATE)?;
            output.SetUINT32(
                &MF_MT_MPEG2_PROFILE,
                eAVEncH264VProfile_ConstrainedBase.0 as u32,
            )?;
            transform.SetOutputType(0, &output, 0)?;
            let input = video_type(&MFVideoFormat_NV12, width, height)?;
            transform.SetInputType(0, &input, 0)?;

            let codec: Option<ICodecAPI> = transform.cast().ok();
            if let Some(codec) = &codec {
                // Nem todo encoder aceita tudo: o que ele recusar fica no padrão dele.
                let _ = codec.SetValue(&CODECAPI_AVLowLatencyMode, &VARIANT::from(1u32));
                let _ = codec.SetValue(
                    &CODECAPI_AVEncCommonRateControlMode,
                    &VARIANT::from(eAVEncCommonRateControlMode_CBR.0 as u32),
                );
                let _ = codec.SetValue(
                    &CODECAPI_AVEncCommonMeanBitRate,
                    &VARIANT::from(MAX_BITRATE),
                );
                let _ = codec.SetValue(&CODECAPI_AVEncMPVGOPSize, &VARIANT::from(GOP_FRAMES));
            }

            let info = transform.GetOutputStreamInfo(0)?;
            transform.ProcessMessage(MFT_MESSAGE_NOTIFY_BEGIN_STREAMING, 0)?;
            transform.ProcessMessage(MFT_MESSAGE_NOTIFY_START_OF_STREAM, 0)?;

            Ok(Self {
                activate: candidate.activate.clone(),
                transform,
                events,
                codec,
                provides_samples: info.dwFlags & MFT_OUTPUT_STREAM_PROVIDES_SAMPLES.0 as u32 != 0,
                output_size: info.cbSize.max(1 << 20),
                d3d,
                textures: mode == InputMode::D3dTexture,
                width,
                height,
                needs: 0,
            })
        }
    }

    /// Codifica quadros pretos até sair o primeiro resultado.
    fn self_test(&mut self) -> Result<(), EncoderError> {
        let luma = (self.width * self.height) as usize;
        let mut frame = vec![0u8; luma + luma / 2];
        frame[luma..].fill(128);
        let start = Instant::now();
        let mut fed = 0i64;
        while start.elapsed() < SELF_TEST {
            match self.next_event()? {
                Some(kind) if kind == METransformHaveOutput => {
                    if self.take_output()?.is_some() {
                        return Ok(());
                    }
                }
                Some(_) => {}
                None => std::thread::sleep(IDLE),
            }
            if self.needs > 0 {
                self.feed(&frame, fed * 33_333)?;
                fed += 1;
            }
        }
        Err(EncoderError::Unavailable)
    }

    /// O próximo evento do encoder, sem esperar. Conta os pedidos de quadro.
    fn next_event(&mut self) -> Result<Option<MF_EVENT_TYPE>, EncoderError> {
        // SAFETY: `events` pertence a esta thread, e o evento devolvido é só lido.
        unsafe {
            match self.events.GetEvent(MF_EVENT_FLAG_NO_WAIT) {
                Ok(event) => {
                    let kind = MF_EVENT_TYPE(event.GetType()? as i32);
                    if kind == METransformNeedInput {
                        self.needs += 1;
                    }
                    Ok(Some(kind))
                }
                Err(err) if err.code() == MF_E_NO_EVENTS_AVAILABLE => Ok(None),
                Err(err) => Err(err.into()),
            }
        }
    }

    fn feed(&mut self, data: &[u8], timestamp_us: i64) -> Result<(), EncoderError> {
        // SAFETY: o tamanho copiado é o do quadro e cabe no buffer criado com esse tamanho, e o
        // buffer fica travado só durante a cópia.
        unsafe {
            let buffer = if self.textures {
                self.texture_buffer(data)?
            } else {
                let buffer = MFCreateMemoryBuffer(data.len() as u32)?;
                let mut pointer = std::ptr::null_mut();
                buffer.Lock(&mut pointer, None, None)?;
                std::ptr::copy_nonoverlapping(data.as_ptr(), pointer, data.len());
                buffer.Unlock()?;
                buffer.SetCurrentLength(data.len() as u32)?;
                buffer
            };
            let sample = MFCreateSample()?;
            sample.AddBuffer(&buffer)?;
            sample.SetSampleTime(timestamp_us * 10)?;
            sample.SetSampleDuration(10_000_000 / i64::from(FRAMERATE))?;
            self.transform.ProcessInput(0, &sample, 0)?;
        }
        self.needs -= 1;
        Ok(())
    }

    /// # Safety
    /// `data` precisa ter o tamanho de um quadro NV12 deste encoder.
    unsafe fn texture_buffer(&self, data: &[u8]) -> Result<IMFMediaBuffer, EncoderError> {
        let d3d = self
            .d3d
            .as_ref()
            .ok_or(windows::core::Error::from(E_FAIL))?;
        let desc = D3D11_TEXTURE2D_DESC {
            Width: self.width,
            Height: self.height,
            MipLevels: 1,
            ArraySize: 1,
            Format: DXGI_FORMAT_NV12,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
            CPUAccessFlags: 0,
            MiscFlags: 0,
        };
        // SAFETY: o dispositivo é desta thread, e `data` tem o tamanho descrito em `desc`.
        unsafe {
            let mut texture: Option<ID3D11Texture2D> = None;
            d3d.device
                .CreateTexture2D(&desc, None, Some(&mut texture))?;
            let texture = texture.ok_or(windows::core::Error::from(E_FAIL))?;
            d3d.context.UpdateSubresource(
                &texture,
                0,
                None,
                data.as_ptr().cast(),
                self.width,
                data.len() as u32,
            );
            let buffer = MFCreateDXGISurfaceBuffer(&ID3D11Texture2D::IID, &texture, 0, false)?;
            buffer.SetCurrentLength(data.len() as u32)?;
            Ok(buffer)
        }
    }

    fn take_output(&mut self) -> Result<Option<EncodedUnit>, EncoderError> {
        // SAFETY: os campos `ManuallyDrop` são retirados uma vez depois de `ProcessOutput`, e o
        // buffer da amostra fica travado só durante a cópia, com o tamanho informado por ele.
        unsafe {
            let mut data = [MFT_OUTPUT_DATA_BUFFER::default()];
            if !self.provides_samples {
                let sample = MFCreateSample()?;
                sample.AddBuffer(&MFCreateMemoryBuffer(self.output_size)?)?;
                data[0].pSample = ManuallyDrop::new(Some(sample));
            }
            let mut status = 0u32;
            let result = self.transform.ProcessOutput(0, &mut data, &mut status);
            let sample = ManuallyDrop::take(&mut data[0].pSample);
            let _events = ManuallyDrop::take(&mut data[0].pEvents);
            match result {
                Ok(()) => {}
                Err(err) if err.code() == MF_E_TRANSFORM_NEED_MORE_INPUT => return Ok(None),
                Err(err) if err.code() == MF_E_TRANSFORM_STREAM_CHANGE => {
                    let output = self.transform.GetOutputAvailableType(0, 0)?;
                    self.transform.SetOutputType(0, &output, 0)?;
                    return Ok(None);
                }
                Err(err) => return Err(err.into()),
            }
            let Some(sample) = sample else {
                return Ok(None);
            };
            let buffer = sample.ConvertToContiguousBuffer()?;
            let mut pointer = std::ptr::null_mut();
            let mut length = 0u32;
            buffer.Lock(&mut pointer, None, Some(&mut length))?;
            let payload = std::slice::from_raw_parts(pointer, length as usize).to_vec();
            buffer.Unlock()?;
            Ok(Some(EncodedUnit {
                payload,
                key: sample.GetUINT32(&MFSampleExtension_CleanPoint).unwrap_or(0) != 0,
                width: self.width,
                height: self.height,
                timestamp_us: sample.GetSampleTime().unwrap_or(0) / 10,
            }))
        }
    }

    fn force_keyframe(&self) {
        if let Some(codec) = &self.codec {
            // SAFETY: `codec` é desta thread, e o `VARIANT` vive durante a chamada.
            let _ =
                unsafe { codec.SetValue(&CODECAPI_AVEncVideoForceKeyFrame, &VARIANT::from(1u32)) };
        }
    }

    fn set_bitrate(&self, bitrate: u32) {
        if let Some(codec) = &self.codec {
            // SAFETY: `codec` é desta thread, e o `VARIANT` vive durante a chamada.
            let _ = unsafe {
                codec.SetValue(&CODECAPI_AVEncCommonMeanBitRate, &VARIANT::from(bitrate))
            };
        }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        // SAFETY: os objetos são desta thread e continuam válidos até o fim deste `drop`.
        unsafe {
            let _ = self
                .transform
                .ProcessMessage(MFT_MESSAGE_NOTIFY_END_OF_STREAM, 0);
            let _ = self
                .transform
                .ProcessMessage(MFT_MESSAGE_NOTIFY_END_STREAMING, 0);
            let _ = self.activate.ShutdownObject();
        }
    }
}

/// # Safety
/// Precisa rodar em uma thread com Media Foundation iniciado.
unsafe fn video_type(
    subtype: &GUID,
    width: u32,
    height: u32,
) -> Result<IMFMediaType, EncoderError> {
    // SAFETY: o tipo é criado aqui e só recebe valores simples.
    unsafe {
        let media_type = MFCreateMediaType()?;
        media_type.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)?;
        media_type.SetGUID(&MF_MT_SUBTYPE, subtype)?;
        media_type.SetUINT64(&MF_MT_FRAME_SIZE, pack(width, height))?;
        media_type.SetUINT64(&MF_MT_FRAME_RATE, pack(FRAMERATE, 1))?;
        media_type.SetUINT64(&MF_MT_PIXEL_ASPECT_RATIO, pack(1, 1))?;
        media_type.SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32)?;
        Ok(media_type)
    }
}

/// Associa o encoder a um dispositivo D3D11 da placa dele.
///
/// # Safety
/// Precisa rodar em uma thread com COM iniciado, com `transform` ainda sem tipos definidos.
unsafe fn open_d3d(candidate: &Candidate, transform: &IMFTransform) -> Result<D3d, EncoderError> {
    // SAFETY: os objetos são criados aqui, e o ponteiro do gerente é válido durante a mensagem,
    // que guarda a própria referência.
    unsafe {
        let factory: IDXGIFactory1 = CreateDXGIFactory1()?;
        let mut adapter: Option<IDXGIAdapter> = None;
        let mut index = 0;
        while let Ok(current) = factory.EnumAdapters1(index) {
            let description = current.GetDesc1()?;
            if candidate.vendor_id == Some(description.VendorId) {
                adapter = Some(current.cast()?);
                break;
            }
            index += 1;
        }
        let mut device = None;
        let mut context = None;
        D3D11CreateDevice(
            adapter.as_ref(),
            D3D_DRIVER_TYPE_UNKNOWN,
            HMODULE::default(),
            D3D11_CREATE_DEVICE_VIDEO_SUPPORT | D3D11_CREATE_DEVICE_BGRA_SUPPORT,
            None,
            D3D11_SDK_VERSION,
            Some(&mut device),
            None,
            Some(&mut context),
        )?;
        let device: ID3D11Device = device.ok_or(windows::core::Error::from(E_FAIL))?;
        let context = context.ok_or(windows::core::Error::from(E_FAIL))?;
        let mut token = 0u32;
        let mut manager = None;
        MFCreateDXGIDeviceManager(&mut token, &mut manager)?;
        let manager: IMFDXGIDeviceManager = manager.ok_or(windows::core::Error::from(E_FAIL))?;
        manager.ResetDevice(&device, token)?;
        transform.ProcessMessage(MFT_MESSAGE_SET_D3D_MANAGER, manager.as_raw() as usize)?;
        Ok(D3d {
            device,
            context,
            _manager: manager,
        })
    }
}

/// Os encoders em hardware deste PC, para diagnóstico e para os testes.
pub fn available_encoders() -> Vec<String> {
    // SAFETY: inicia e encerra COM e Media Foundation nesta thread, sem objetos sobrando.
    std::thread::spawn(|| unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let names = if MFStartup(MF_VERSION, MFSTARTUP_FULL).is_ok() {
            let names = enumerate_encoders().into_iter().map(|c| c.name).collect();
            let _ = MFShutdown();
            names
        } else {
            Vec::new()
        };
        CoUninitialize();
        names
    })
    .join()
    .unwrap_or_default()
}
