//! O microfone só é liberado às origens do próprio app (ADR 0015).

use moon_lib::mic_permission::is_app_origin;

#[test]
fn accepts_build_origins() {
    assert!(is_app_origin("http://tauri.localhost"));
    assert!(is_app_origin("http://tauri.localhost/"));
    assert!(is_app_origin("http://tauri.localhost/index.html"));
    assert!(is_app_origin("tauri://localhost/"));
}

#[test]
fn accepts_dev_origins_only_in_dev_builds() {
    for origin in ["http://localhost:1420/", "http://localhost:1439/"] {
        assert_eq!(is_app_origin(origin), cfg!(debug_assertions));
    }
}

#[test]
fn rejects_other_origins() {
    for origin in [
        "http://localhost:1419/",
        "http://localhost:1440/",
        "http://localhost/",
        "http://tauri.localhost.evil.example/",
        "http://tauri.localhost@evil.example/",
        "https://tauri.localhost/",
        "http://evil.example/http://tauri.localhost/",
        "",
    ] {
        assert!(!is_app_origin(origin), "deveria recusar {origin}");
    }
}
