//! Atualização do app pelo GitHub Releases, sem chaves de assinatura (ADR 0019).

use semver::Version;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::path::PathBuf;
use std::time::Duration;
use tauri::AppHandle;

const LATEST_RELEASE_URL: &str =
    "https://api.github.com/repos/andreramosdovale/orbit/releases/latest";
const RELEASE_DOWNLOAD_URL: &str = "https://github.com/andreramosdovale/orbit/releases/download";
/// A API do GitHub recusa pedidos sem `User-Agent`.
const USER_AGENT: &str = "orbit-moon";
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15 * 60);
/// Instala só com a barra de progresso (`/P`), mantendo os atalhos (`/UPDATE`), e reabre (`/R`).
const INSTALLER_ARGS: [&str; 3] = ["/P", "/UPDATE", "/R"];

/// Uma versão mais nova que a instalada.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct AvailableUpdate {
    pub version: String,
}

#[derive(Deserialize)]
struct LatestRelease {
    tag_name: String,
}

/// A versão da tag `v<versão>`, se for estável e mais nova que `current`.
pub fn newer_version(tag: &str, current: &str) -> Option<String> {
    let latest = Version::parse(tag.strip_prefix('v')?).ok()?;
    let current = Version::parse(current).ok()?;
    (latest.pre.is_empty() && latest > current).then(|| latest.to_string())
}

pub fn installer_file_name(version: &str) -> String {
    format!("Orbit_{version}_x64-setup.exe")
}

pub fn installer_url(version: &str) -> String {
    format!(
        "{RELEASE_DOWNLOAD_URL}/v{version}/{}",
        installer_file_name(version)
    )
}

fn http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(|err| format!("Não foi possível preparar a conexão: {err}"))
}

async fn find_update(client: &reqwest::Client) -> Result<Option<AvailableUpdate>, String> {
    // Em dev, uma atualização trocaria o app instalado, e não o que está sendo testado.
    if cfg!(debug_assertions) {
        return Ok(None);
    }

    let check_failed =
        |err: reqwest::Error| format!("Não foi possível procurar atualizações: {err}");
    let response = client
        .get(LATEST_RELEASE_URL)
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .send()
        .await
        .map_err(check_failed)?;
    // Nenhum release publicado ainda.
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    let release: LatestRelease = response
        .error_for_status()
        .map_err(check_failed)?
        .json()
        .await
        .map_err(check_failed)?;

    Ok(newer_version(&release.tag_name, env!("CARGO_PKG_VERSION"))
        .map(|version| AvailableUpdate { version }))
}

async fn download_installer(client: &reqwest::Client, version: &str) -> Result<PathBuf, String> {
    let download_failed =
        |err: reqwest::Error| format!("Não foi possível baixar a versão {version}: {err}");
    let installer = client
        .get(installer_url(version))
        .send()
        .await
        .map_err(download_failed)?
        .error_for_status()
        .map_err(download_failed)?
        .bytes()
        .await
        .map_err(download_failed)?;

    let path = std::env::temp_dir().join(installer_file_name(version));
    tokio::fs::write(&path, &installer)
        .await
        .map_err(|err| format!("Não foi possível salvar o instalador: {err}"))?;
    Ok(path)
}

/// Procura uma versão nova. Em build de desenvolvimento, nunca encontra.
#[tauri::command]
#[specta::specta]
pub async fn check_for_update() -> Result<Option<AvailableUpdate>, String> {
    find_update(&http_client()?).await
}

/// Baixa a versão mais recente, abre o instalador e fecha o app. O instalador reabre o app.
#[tauri::command]
#[specta::specta]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    let client = http_client()?;
    let update = find_update(&client)
        .await?
        .ok_or("Não há uma versão nova para instalar.")?;
    let installer = download_installer(&client, &update.version).await?;
    std::process::Command::new(&installer)
        .args(INSTALLER_ARGS)
        .spawn()
        .map_err(|err| format!("Não foi possível abrir o instalador: {err}"))?;
    app.exit(0);
    Ok(())
}
