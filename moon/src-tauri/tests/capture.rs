//! Testes da captura. Os marcados com `ignore` precisam de áudio e de uma área de trabalho:
//! rode com `cargo test -p moon --test capture -- --ignored --test-threads=1`.

use moon_lib::capture::audio::{self, AudioChunker};
use moon_lib::capture::encoder::{
    EncodedUnit, EncoderOutput, HardwareEncoder, Nv12Frame, available_encoders, clamp_bitrate,
    parse_vendor_id, vendor_rank,
};
use moon_lib::capture::video::{self, FrameSink, fit_size};
use moon_lib::capture::{CaptureTarget, list_targets, offered_targets};
use std::f64::consts::TAU;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

fn window(title: &str, process_id: u32) -> CaptureTarget {
    CaptureTarget::Window {
        id: "1".into(),
        title: title.into(),
        app: "app.exe".into(),
        process_id,
    }
}

#[test]
fn offered_targets_hides_the_app_itself_and_untitled_windows() {
    let screen = CaptureTarget::Screen {
        index: 1,
        name: "Tela".into(),
    };
    let offered = offered_targets(
        vec![
            window("Jogo", 10),
            window("Orbit", 99),
            window("   ", 10),
            screen.clone(),
        ],
        99,
    );
    assert_eq!(offered, vec![window("Jogo", 10), screen]);
}

#[test]
fn fit_size_keeps_small_sizes_and_shrinks_large_ones() {
    assert_eq!(fit_size(1280, 720), (1280, 720));
    assert_eq!(fit_size(1920, 1080), (1920, 1080));
    assert_eq!(fit_size(3840, 2160), (1920, 1080));
    assert_eq!(fit_size(1920, 1200), (1728, 1080));
    assert_eq!(fit_size(1281, 721), (1280, 720));
    assert_eq!(fit_size(1, 1), (2, 2));
}

#[test]
fn chunker_makes_frames_of_ten_ms_and_keeps_the_rest() {
    let mut chunker = AudioChunker::default();
    let samples = (audio::SAMPLES_PER_CHANNEL * audio::CHANNELS) as usize;
    let bytes: Vec<u8> = (0..samples + 1)
        .flat_map(|_| 0.5f32.to_le_bytes())
        .collect();

    chunker.queue_mut().extend(&bytes[..bytes.len() - 8]);
    assert!(chunker.next_frame().is_none(), "ainda não tem 10 ms");

    chunker.queue_mut().extend(&bytes[bytes.len() - 8..]);
    let frame = chunker.next_frame().expect("deveria ter um bloco");
    assert_eq!(frame.len(), samples);
    assert_eq!(frame[0], (0.5 * f32::from(i16::MAX)) as i16);
    assert!(chunker.next_frame().is_none(), "sobrou menos de um bloco");
}

#[test]
fn chunker_limits_loud_samples() {
    let mut chunker = AudioChunker::default();
    let samples = (audio::SAMPLES_PER_CHANNEL * audio::CHANNELS) as usize;
    for _ in 0..samples {
        chunker.queue_mut().extend(2.0f32.to_le_bytes());
    }
    assert_eq!(chunker.next_frame().expect("bloco")[0], i16::MAX);
}

/// Um WAV de um tom baixo: serve para o detector, e quase não se ouve.
fn write_tone(path: &Path, frequency: f64) {
    const RATE: u32 = 48_000;
    let samples: Vec<i16> = (0..RATE)
        .map(|n| (f64::sin(TAU * frequency * f64::from(n) / f64::from(RATE)) * 400.0) as i16)
        .collect();
    let data_len = (samples.len() * 2) as u32;
    let mut wav = Vec::with_capacity(44 + data_len as usize);
    wav.extend(b"RIFF");
    wav.extend((36 + data_len).to_le_bytes());
    wav.extend(b"WAVEfmt ");
    wav.extend(16u32.to_le_bytes());
    wav.extend(1u16.to_le_bytes());
    wav.extend(1u16.to_le_bytes());
    wav.extend(RATE.to_le_bytes());
    wav.extend((RATE * 2).to_le_bytes());
    wav.extend(2u16.to_le_bytes());
    wav.extend(16u16.to_le_bytes());
    wav.extend(b"data");
    wav.extend(data_len.to_le_bytes());
    for sample in samples {
        wav.extend(sample.to_le_bytes());
    }
    fs::write(path, wav).unwrap_or_else(|err| panic!("escrever o WAV: {err}"));
}

fn fixture_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("orbit-capture-{name}-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap_or_else(|err| panic!("criar a pasta: {err}"));
    dir
}

struct Killed(Child);
impl Drop for Killed {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn powershell(script: &str) -> Killed {
    Killed(
        Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", script])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap_or_else(|err| panic!("abrir o PowerShell: {err}")),
    )
}

fn play_tone(path: &Path) -> Killed {
    powershell(&format!(
        "$p = New-Object System.Media.SoundPlayer '{}'; $p.PlayLooping(); Start-Sleep 60",
        path.display()
    ))
}

/// A força da frequência no sinal (algoritmo de Goertzel), só no canal esquerdo.
fn strength(samples: &[i16], frequency: f64) -> f64 {
    let omega = TAU * frequency / f64::from(audio::SAMPLE_RATE);
    let coeff = 2.0 * omega.cos();
    let (mut previous, mut before) = (0.0, 0.0);
    let mut count: f64 = 0.0;
    for frame in samples.as_chunks::<2>().0 {
        let value = f64::from(frame[0]) + coeff * previous - before;
        before = previous;
        previous = value;
        count += 1.0;
    }
    (before * before + previous * previous - coeff * previous * before).sqrt() / count.max(1.0)
}

/// Prova da spec 013: só o áudio do processo escolhido é capturado.
#[test]
#[ignore = "precisa de um dispositivo de áudio"]
fn loopback_captures_only_the_chosen_process() {
    let dir = fixture_dir("audio");
    let (chosen_wav, other_wav) = (dir.join("chosen.wav"), dir.join("other.wav"));
    write_tone(&chosen_wav, 1000.0);
    write_tone(&other_wav, 1500.0);

    let chosen = play_tone(&chosen_wav);
    let _other = play_tone(&other_wav);
    std::thread::sleep(Duration::from_secs(2));

    let stop = Arc::new(AtomicBool::new(false));
    let captured = Arc::new(Mutex::new(Vec::<i16>::new()));
    let (thread_stop, thread_captured) = (stop.clone(), captured.clone());
    let process_id = chosen.0.id();
    let thread = std::thread::spawn(move || {
        audio::capture_process(process_id, &thread_stop, |frame| {
            thread_captured.lock().expect("trava").extend(frame);
        })
    });
    std::thread::sleep(Duration::from_secs(4));
    stop.store(true, Ordering::Relaxed);
    thread.join().expect("thread").expect("captura");

    let samples = captured.lock().expect("trava");
    let (wanted, unwanted) = (strength(&samples, 1000.0), strength(&samples, 1500.0));
    println!(
        "amostras={} 1000Hz={wanted:.1} 1500Hz={unwanted:.1}",
        samples.len()
    );
    assert!(samples.len() > 48_000, "deveria ter áudio");
    assert!(wanted > 20.0, "o tom escolhido deveria estar no áudio");
    assert!(
        unwanted < wanted / 20.0,
        "o outro processo não deveria entrar no áudio"
    );
    let _ = fs::remove_dir_all(dir);
}

const FIXTURE_TITLE: &str = "Orbit Janela de Teste";

fn open_fixture_window() -> Killed {
    powershell(&format!(
        "Add-Type -AssemblyName System.Windows.Forms; \
         $f = New-Object Windows.Forms.Form; $f.Text = '{FIXTURE_TITLE}'; $f.Width = 640; $f.Height = 360; \
         $t = New-Object Windows.Forms.Timer; $t.Interval = 50; $script:n = 0; \
         $t.Add_Tick({{ $script:n = ($script:n + 8) % 255; $f.BackColor = [Drawing.Color]::FromArgb(0, $script:n, 0) }}); \
         $t.Start(); [Windows.Forms.Application]::Run($f)"
    ))
}

fn find_fixture() -> Option<CaptureTarget> {
    list_targets().into_iter().find(
        |target| matches!(target, CaptureTarget::Window { title, .. } if title == FIXTURE_TITLE),
    )
}

/// Prova da spec 013: a janela chega em quadros, e fechá-la avisa o fim.
#[test]
#[ignore = "precisa de uma área de trabalho"]
fn window_capture_delivers_frames_and_reports_closing() {
    let mut fixture = open_fixture_window();
    let start = Instant::now();
    let target = loop {
        if let Some(target) = find_fixture() {
            break target;
        }
        assert!(
            start.elapsed() < Duration::from_secs(20),
            "a janela de teste não apareceu"
        );
        std::thread::sleep(Duration::from_millis(200));
    };

    let frames = Arc::new(AtomicU64::new(0));
    let size = Arc::new(Mutex::new((0, 0)));
    let closed = Arc::new(AtomicBool::new(false));
    let (on_frames, on_size, on_closed) = (frames.clone(), size.clone(), closed.clone());
    let capture = video::start(
        &target,
        FrameSink {
            on_frame: Box::new(move |buffer| {
                use livekit::webrtc::prelude::VideoBuffer;
                on_frames.fetch_add(1, Ordering::Relaxed);
                *on_size.lock().expect("trava") = (buffer.width(), buffer.height());
            }),
            on_ended: Some(Box::new(move |_| on_closed.store(true, Ordering::SeqCst))),
        },
    )
    .expect("iniciar a captura");

    std::thread::sleep(Duration::from_secs(3));
    let count = frames.load(Ordering::Relaxed);
    let measured = *size.lock().expect("trava");
    println!("quadros em 3s={count} tamanho={measured:?}");
    assert!(count > 10, "deveria receber quadros");
    assert!(
        measured.0 >= 600 && measured.1 >= 300,
        "tamanho inesperado: {measured:?}"
    );

    let _ = fixture.0.kill();
    let _ = fixture.0.wait();
    let start = Instant::now();
    while !closed.load(Ordering::SeqCst) && start.elapsed() < Duration::from_secs(5) {
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(
        closed.load(Ordering::SeqCst),
        "fechar a janela deveria avisar o fim"
    );
    capture.stop();
}

#[test]
fn nv12_keeps_luma_and_interleaves_chroma() {
    use livekit::webrtc::prelude::I420Buffer;
    let mut buffer = I420Buffer::new(4, 2);
    {
        let (y, u, v) = buffer.data_mut();
        y.fill(10);
        u.fill(20);
        v.fill(30);
    }
    let frame = Nv12Frame::from_i420(&buffer);
    assert_eq!((frame.width, frame.height), (4, 2));
    assert_eq!(frame.data, [10, 10, 10, 10, 10, 10, 10, 10, 20, 30, 20, 30]);
}

#[test]
fn bitrate_requests_stay_within_the_limits() {
    assert_eq!(clamp_bitrate(0), 300_000);
    assert_eq!(clamp_bitrate(1_500_000), 1_500_000);
    assert_eq!(clamp_bitrate(50_000_000), 4_000_000);
}

#[test]
fn encoders_of_dedicated_cards_come_first() {
    assert_eq!(parse_vendor_id("VEN_10DE"), Some(0x10DE));
    assert_eq!(parse_vendor_id("outra coisa"), None);
    assert!(vendor_rank(Some(0x10DE)) < vendor_rank(Some(0x1002)));
    assert!(vendor_rank(Some(0x1002)) < vendor_rank(Some(0x8086)));
    assert!(vendor_rank(Some(0x8086)) < vendor_rank(None));
}

#[derive(Default)]
struct Seen {
    units: u64,
    keys: u64,
    bytes: u64,
    first_was_key: Option<bool>,
    last_size: (u32, u32),
    failed: bool,
}

#[derive(Clone, Default)]
struct TestOutput {
    seen: Arc<Mutex<Seen>>,
    want_key: Arc<AtomicBool>,
    bitrate: Arc<Mutex<Option<u32>>>,
}

impl EncoderOutput for TestOutput {
    fn on_unit(&mut self, unit: &EncodedUnit) {
        let mut seen = self.seen.lock().unwrap_or_else(|err| err.into_inner());
        seen.units += 1;
        seen.keys += u64::from(unit.key);
        seen.bytes += unit.payload.len() as u64;
        seen.first_was_key.get_or_insert(unit.key);
        seen.last_size = (unit.width, unit.height);
    }
    fn take_keyframe_request(&mut self) -> bool {
        self.want_key.swap(false, Ordering::SeqCst)
    }
    fn take_bitrate_request(&mut self) -> Option<u32> {
        self.bitrate
            .lock()
            .unwrap_or_else(|err| err.into_inner())
            .take()
    }
    fn on_failed(&mut self) {
        self.seen
            .lock()
            .unwrap_or_else(|err| err.into_inner())
            .failed = true;
    }
}

/// Um quadro difícil de comprimir, para o encoder precisar respeitar a taxa.
fn noisy_frame(width: u32, height: u32, seed: u64) -> Nv12Frame {
    let mut state = seed | 1;
    let luma = (width * height) as usize;
    let mut data = vec![128u8; luma + luma / 2];
    for pixel in &mut data[..luma] {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        *pixel = state as u8;
    }
    Nv12Frame {
        data,
        width,
        height,
    }
}

fn feed_for(
    encoder: &HardwareEncoder,
    width: u32,
    height: u32,
    duration: Duration,
    seed: &mut u64,
) {
    let start = Instant::now();
    while start.elapsed() < duration {
        *seed += 1;
        encoder.push(noisy_frame(width, height, *seed));
        std::thread::sleep(Duration::from_millis(33));
    }
}

/// Prova da spec 014, com o encoder real deste PC.
#[test]
#[ignore = "precisa de um encoder em hardware"]
fn hardware_encoder_encodes_obeys_requests_and_resizes() {
    println!("encoders: {:?}", available_encoders());
    let output = TestOutput::default();
    let encoder = HardwareEncoder::start(1280, 720, output.clone()).expect("abrir o encoder");
    println!("usando: {}", encoder.name());
    let mut seed = 1;

    feed_for(&encoder, 1280, 720, Duration::from_secs(3), &mut seed);
    let high = {
        let seen = output.seen.lock().expect("trava");
        println!(
            "3s: quadros={} chaves={} bytes={}",
            seen.units, seen.keys, seen.bytes
        );
        assert!(seen.units > 40, "deveria codificar quadros");
        assert_eq!(
            seen.first_was_key,
            Some(true),
            "o primeiro quadro deveria ser chave"
        );
        seen.bytes as f64 / 3.0
    };

    let keys_before = output.seen.lock().expect("trava").keys;
    output.want_key.store(true, Ordering::SeqCst);
    feed_for(&encoder, 1280, 720, Duration::from_millis(500), &mut seed);
    assert!(
        output.seen.lock().expect("trava").keys > keys_before,
        "o pedido de quadro-chave deveria ser atendido"
    );

    *output.bitrate.lock().expect("trava") = Some(500_000);
    feed_for(&encoder, 1280, 720, Duration::from_secs(1), &mut seed);
    let before = output.seen.lock().expect("trava").bytes;
    feed_for(&encoder, 1280, 720, Duration::from_secs(3), &mut seed);
    let low = (output.seen.lock().expect("trava").bytes - before) as f64 / 3.0;
    println!(
        "taxa: alta={:.2} Mbps baixa={:.2} Mbps",
        high * 8.0 / 1e6,
        low * 8.0 / 1e6
    );
    assert!(low < high / 2.0, "reduzir a banda deveria baixar a taxa");

    feed_for(&encoder, 640, 360, Duration::from_secs(2), &mut seed);
    {
        let seen = output.seen.lock().expect("trava");
        assert_eq!(
            seen.last_size,
            (640, 360),
            "deveria recriar o encoder no novo tamanho"
        );
        assert!(!seen.failed);
    }
    encoder.stop();
}
