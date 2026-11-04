//! Lado Rust do app. Comandos expostos ao front são registrados em [`specta_builder`],
//! que também gera `src/bindings.ts` (ver docs/decisions/0005).

pub mod capture;
pub mod mic_permission;

use orbit_shared::{
    ApiErrorBody, ChatMessage, ClientMessage, CreateChannelRequest, CreateVoiceChannelRequest,
    LoginRequest, LoginResponse, MessagePage, MessageRejectReason, RegisterRequest, ServerMessage,
    TextChannel, User, UserPresence, VoiceChannel, VoiceJoin,
};
use specta_typescript::Typescript;
use tauri_specta::{Builder, collect_commands, collect_events};

/// Caminho do arquivo de tipos gerado, relativo a `src-tauri/`.
const BINDINGS_PATH: &str = "../src/bindings.ts";

/// Versão do app, para exibir na UI.
#[tauri::command]
#[specta::specta]
fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Registro único de comandos e tipos expostos ao front.
fn specta_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            app_version,
            capture::commands::list_capture_targets,
            capture::commands::start_screen_share,
            capture::commands::stop_screen_share,
        ])
        .events(collect_events![capture::commands::ScreenShareEnded])
        // Tipos do protocolo usados direto pelo front no WebSocket.
        .typ::<ClientMessage>()
        .typ::<ServerMessage>()
        // Tipos da API REST do `sun` usados pelo front via `fetch`.
        .typ::<RegisterRequest>()
        .typ::<User>()
        .typ::<UserPresence>()
        .typ::<LoginRequest>()
        .typ::<LoginResponse>()
        .typ::<TextChannel>()
        .typ::<ChatMessage>()
        .typ::<MessagePage>()
        .typ::<MessageRejectReason>()
        .typ::<CreateChannelRequest>()
        .typ::<VoiceChannel>()
        .typ::<CreateVoiceChannelRequest>()
        .typ::<VoiceJoin>()
        .typ::<ApiErrorBody>()
}

/// Gera `src/bindings.ts`. Roda em todo `tauri dev` e no teste `tests/bindings.rs`.
pub fn export_bindings() -> Result<(), specta_typescript::Error> {
    specta_builder().export(Typescript::default(), BINDINGS_PATH)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = specta_builder();

    // Em dev, bindings desatualizados não devem impedir o app de abrir: o teste
    // `tests/bindings.rs` (e a CI) é quem garante que o arquivo está certo.
    #[cfg(debug_assertions)]
    if let Err(err) = export_bindings() {
        eprintln!("aviso: falha ao exportar bindings TypeScript: {err}");
    }

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(capture::commands::ScreenShareState::default())
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            builder.mount_events(app);
            #[cfg(windows)]
            {
                use tauri::Manager;
                if let Some(window) = app.get_webview_window("main") {
                    mic_permission::allow_microphone(&window)?;
                }
            }
            Ok(())
        })
        .build(tauri::generate_context!());

    let app = match app {
        Ok(app) => app,
        Err(err) => {
            eprintln!("erro ao executar o app: {err}");
            std::process::exit(1);
        }
    };
    app.run(|handle, event| {
        // Fechar o app encerra a transmissão, para ela sair na hora da sala.
        if let tauri::RunEvent::Exit = event {
            use tauri::Manager;
            let state = handle.state::<capture::commands::ScreenShareState>();
            tauri::async_runtime::block_on(state.stop());
        }
    });
}
