//! Transmissão de tela nativa (spec 013, ADR 0017): captura da janela ou da tela, áudio
//! só do aplicativo escolhido e publicação no LiveKit como uma identidade separada.

pub mod audio;
pub mod commands;
pub mod publish;
pub mod video;

use serde::{Deserialize, Serialize};
use specta::Type;
use windows_capture::monitor::Monitor;
use windows_capture::window::Window;

/// O que dá para transmitir.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CaptureTarget {
    /// Uma janela, com o áudio do aplicativo dela.
    Window {
        /// O `HWND`, em texto: não cabe em um número do TypeScript com segurança.
        id: String,
        title: String,
        app: String,
        process_id: u32,
    },
    /// Uma tela inteira, sem áudio.
    Screen {
        /// Índice do monitor, a partir de 1.
        index: u32,
        name: String,
    },
}

/// Tira o que não faz sentido oferecer: janelas sem título e as do próprio Orbit, que
/// mostrariam a si mesmas e levariam junto as vozes da sala.
pub fn offered_targets(targets: Vec<CaptureTarget>, own_process_id: u32) -> Vec<CaptureTarget> {
    targets
        .into_iter()
        .filter(|target| match target {
            CaptureTarget::Window {
                title, process_id, ..
            } => !title.trim().is_empty() && *process_id != own_process_id,
            CaptureTarget::Screen { .. } => true,
        })
        .collect()
}

/// As janelas e telas abertas agora.
pub fn list_targets() -> Vec<CaptureTarget> {
    let mut targets = Vec::new();
    for window in Window::enumerate().unwrap_or_default() {
        let (Ok(title), Ok(process_id)) = (window.title(), window.process_id()) else {
            continue;
        };
        targets.push(CaptureTarget::Window {
            id: (window.as_raw_hwnd() as usize).to_string(),
            title,
            app: window.process_name().unwrap_or_default(),
            process_id,
        });
    }
    for monitor in Monitor::enumerate().unwrap_or_default() {
        let Ok(index) = monitor.index() else { continue };
        targets.push(CaptureTarget::Screen {
            index: index as u32,
            name: monitor.name().unwrap_or_else(|_| format!("Tela {index}")),
        });
    }
    offered_targets(targets, std::process::id())
}
