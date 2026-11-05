//! O instalador e a atualização: o que os apps já instalados esperam encontrar (ADR 0019).

use moon_lib::update::{installer_url, newer_version};
use serde_json::{Value, json};

fn tauri_conf() -> Value {
    serde_json::from_str(include_str!("../tauri.conf.json"))
        .unwrap_or_else(|err| panic!("tauri.conf.json inválido: {err}"))
}

#[test]
fn only_builds_the_nsis_installer() {
    assert_eq!(tauri_conf()["bundle"]["targets"], json!(["nsis"]));
}

#[test]
fn installs_without_administrator() {
    let conf = tauri_conf();
    assert_eq!(
        conf["bundle"]["windows"]["nsis"]["installMode"],
        "currentUser"
    );
}

#[test]
fn version_comes_only_from_cargo_toml() {
    assert!(tauri_conf().get("version").is_none());
}

#[test]
fn product_name_matches_the_installer_url() {
    assert_eq!(tauri_conf()["productName"], "Orbit");
}

#[test]
fn installer_url_points_to_this_repository() {
    assert_eq!(
        installer_url("1.2.3"),
        "https://github.com/andreramosdovale/orbit/releases/download/v1.2.3/Orbit_1.2.3_x64-setup.exe"
    );
}

#[test]
fn offers_only_newer_versions() {
    assert_eq!(newer_version("v0.2.0", "0.1.0").as_deref(), Some("0.2.0"));
    assert_eq!(newer_version("v0.10.0", "0.9.0").as_deref(), Some("0.10.0"));
    assert_eq!(newer_version("v0.1.0", "0.1.0"), None);
    assert_eq!(newer_version("v0.0.9", "0.1.0"), None);
}

#[test]
fn ignores_prereleases_and_unknown_tags() {
    assert_eq!(newer_version("v0.2.0-beta.1", "0.1.0"), None);
    assert_eq!(newer_version("0.2.0", "0.1.0"), None);
    assert_eq!(newer_version("vqualquer", "0.1.0"), None);
    assert_eq!(newer_version("v0.2.0/../../outro", "0.1.0"), None);
}
