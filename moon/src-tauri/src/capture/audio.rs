//! Áudio de um único aplicativo, por WASAPI process loopback (Windows 10 2004 ou mais novo).

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use wasapi::{
    AudioClient, Direction, SampleType, StreamMode, WasapiError, WaveFormat, initialize_mta,
};

pub const SAMPLE_RATE: u32 = 48_000;
pub const CHANNELS: u32 = 2;
/// O LiveKit recebe o áudio em blocos de 10 ms.
pub const SAMPLES_PER_CHANNEL: u32 = SAMPLE_RATE / 100;
const FRAME_SAMPLES: usize = (SAMPLES_PER_CHANNEL * CHANNELS) as usize;
const BYTES_PER_SAMPLE: usize = 4;
/// Quanto esperar por áudio antes de olhar se é hora de parar.
const WAIT_MS: u32 = 100;

/// Junta os pacotes do WASAPI, em `f32`, em blocos de 10 ms em `i16`.
#[derive(Default)]
pub struct AudioChunker {
    bytes: VecDeque<u8>,
}

impl AudioChunker {
    pub fn queue_mut(&mut self) -> &mut VecDeque<u8> {
        &mut self.bytes
    }

    /// O próximo bloco, se já houver áudio suficiente.
    pub fn next_frame(&mut self) -> Option<Vec<i16>> {
        if self.bytes.len() < FRAME_SAMPLES * BYTES_PER_SAMPLE {
            return None;
        }
        let mut frame = Vec::with_capacity(FRAME_SAMPLES);
        for _ in 0..FRAME_SAMPLES {
            let mut sample = [0u8; BYTES_PER_SAMPLE];
            for byte in &mut sample {
                *byte = self.bytes.pop_front().unwrap_or_default();
            }
            frame.push(to_i16(f32::from_le_bytes(sample)));
        }
        Some(frame)
    }
}

fn to_i16(sample: f32) -> i16 {
    (sample.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16
}

/// Captura o processo, com os filhos dele, até `stop`. Bloqueia: rode em uma thread própria.
pub fn capture_process(
    process_id: u32,
    stop: &Arc<AtomicBool>,
    mut on_frame: impl FnMut(Vec<i16>),
) -> Result<(), WasapiError> {
    // Sem efeito se a thread já estiver em um apartamento: a própria abertura avisaria.
    let _ = initialize_mta();

    let format = WaveFormat::new(
        32,
        32,
        &SampleType::Float,
        SAMPLE_RATE as usize,
        CHANNELS as usize,
        None,
    );
    let mut client = AudioClient::new_application_loopback_client(process_id, true)?;
    let mode = StreamMode::EventsShared {
        autoconvert: true,
        buffer_duration_hns: 0,
    };
    client.initialize_client(&format, &Direction::Capture, &mode)?;
    let event = client.set_get_eventhandle()?;
    let capture = client.get_audiocaptureclient()?;
    client.start_stream()?;

    let mut chunker = AudioChunker::default();
    let result = loop {
        if stop.load(Ordering::Relaxed) {
            break Ok(());
        }
        match event.wait_for_event(WAIT_MS) {
            Ok(()) | Err(WasapiError::EventTimeout) => {}
            Err(err) => break Err(err),
        }
        if let Err(err) = capture.read_from_device_to_deque(chunker.queue_mut()) {
            break Err(err);
        }
        while let Some(frame) = chunker.next_frame() {
            on_frame(frame);
        }
    };
    let _ = client.stop_stream();
    result
}
