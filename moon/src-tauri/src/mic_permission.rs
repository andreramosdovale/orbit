//! Responde ao pedido do microfone, sem o diálogo do WebView2 (ADR 0015). Clicar em
//! "Bloquear" nele não teria volta, e entrar em uma sala já é a autorização.

/// Origens do próprio app: as do build e, em dev, as do Vite (`moon/scripts/dev.mjs`).
pub fn is_app_origin(uri: &str) -> bool {
    let matches_origin = |origin: &str| {
        uri.strip_prefix(origin)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
    };
    if matches_origin("http://tauri.localhost") || matches_origin("tauri://localhost") {
        return true;
    }
    cfg!(debug_assertions) && (1420..=1439).any(|port| matches_origin(&dev_origin(port)))
}

fn dev_origin(port: u16) -> String {
    format!("http://localhost:{port}")
}

/// Passa a permitir o microfone às origens do app. Os outros pedidos seguem o padrão do WebView2.
#[cfg(windows)]
pub fn allow_microphone(window: &tauri::WebviewWindow) -> tauri::Result<()> {
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        COREWEBVIEW2_PERMISSION_KIND_MICROPHONE, COREWEBVIEW2_PERMISSION_KIND_UNKNOWN_PERMISSION,
        COREWEBVIEW2_PERMISSION_STATE_ALLOW, ICoreWebView2PermissionRequestedEventArgs,
    };
    use webview2_com::{PermissionRequestedEventHandler, take_pwstr};

    window.with_webview(|webview| {
        // SAFETY: as chamadas são feitas na thread da interface, com objetos COM válidos
        // entregues pelo Tauri e pelo próprio WebView2, e `take_pwstr` libera a string.
        let registered = unsafe {
            webview.controller().CoreWebView2().and_then(|core| {
                let handler = PermissionRequestedEventHandler::create(Box::new(
                    |_, args: Option<ICoreWebView2PermissionRequestedEventArgs>| {
                        let Some(args) = args else { return Ok(()) };
                        let mut kind = COREWEBVIEW2_PERMISSION_KIND_UNKNOWN_PERMISSION;
                        args.PermissionKind(&mut kind)?;
                        if kind != COREWEBVIEW2_PERMISSION_KIND_MICROPHONE {
                            return Ok(());
                        }
                        let mut uri = Default::default();
                        args.Uri(&mut uri)?;
                        if is_app_origin(&take_pwstr(uri)) {
                            args.SetState(COREWEBVIEW2_PERMISSION_STATE_ALLOW)?;
                        }
                        Ok(())
                    },
                ));
                let mut token = 0;
                core.add_PermissionRequested(&handler, &mut token)
            })
        };
        if let Err(err) = registered {
            // Sem isso, o WebView2 só volta a mostrar o diálogo dele.
            eprintln!("aviso: não foi possível tratar a permissão do microfone: {err}");
        }
    })
}
