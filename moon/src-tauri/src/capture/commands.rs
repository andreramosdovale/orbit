//! Comandos e evento da transmissão, usados pelo front por `bindings.ts`.

use super::publish::{self, EndReason, ScreenShare, VideoEncoderKind};
use super::{CaptureTarget, list_targets};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};
use tauri_specta::Event;
use tokio::sync::Mutex;

/// A transmissão acabou sem ter sido parada.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
pub struct ScreenShareEnded {
    pub reason: EndReason,
}

struct Running {
    /// Distingue esta transmissão de uma mais nova, se o aviso de fim chegar atrasado.
    generation: u64,
    share: ScreenShare,
}

/// A transmissão em andamento, no máximo uma por vez.
#[derive(Default)]
pub struct ScreenShareState {
    running: Arc<Mutex<(u64, Option<Running>)>>,
}

impl ScreenShareState {
    /// Para o que estiver no ar. Usado ao fechar o app.
    pub async fn stop(&self) {
        let running = self.running.lock().await.1.take();
        if let Some(running) = running {
            running.share.stop().await;
        }
    }
}

/// Janelas e telas que dá para transmitir.
#[tauri::command]
#[specta::specta]
pub async fn list_capture_targets() -> Vec<CaptureTarget> {
    tokio::task::spawn_blocking(list_targets)
        .await
        .unwrap_or_default()
}

/// Começa a transmitir, trocando o que já estiver no ar.
#[tauri::command]
#[specta::specta]
pub async fn start_screen_share(
    app: AppHandle,
    state: State<'_, ScreenShareState>,
    target: CaptureTarget,
    url: String,
    token: String,
) -> Result<VideoEncoderKind, String> {
    state.stop().await;

    let generation = {
        let mut running = state.running.lock().await;
        running.0 += 1;
        running.0
    };
    let handle = app.clone();
    let on_ended = move |reason: EndReason| {
        let handle = handle.clone();
        tauri::async_runtime::spawn(async move {
            let state = handle.state::<ScreenShareState>();
            let finished = {
                let mut running = state.running.lock().await;
                let current = running
                    .1
                    .as_ref()
                    .is_some_and(|r| r.generation == generation);
                if current { running.1.take() } else { None }
            };
            if let Some(finished) = finished {
                finished.share.stop().await;
                if let Err(err) = (ScreenShareEnded { reason }).emit(&handle) {
                    eprintln!("aviso: não foi possível avisar o fim da transmissão: {err}");
                }
            }
        });
    };

    let share = publish::start(target, &url, &token, on_ended)
        .await
        .map_err(|err| err.to_string())?;
    let encoder_kind = share.encoder_kind();
    state.running.lock().await.1 = Some(Running { generation, share });
    Ok(encoder_kind)
}

/// Para de transmitir.
#[tauri::command]
#[specta::specta]
pub async fn stop_screen_share(state: State<'_, ScreenShareState>) -> Result<(), String> {
    state.stop().await;
    Ok(())
}
