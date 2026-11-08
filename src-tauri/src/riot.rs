//! Riot Client local integration.
//!
//! Implemented today:
//! - lockfile discovery and parsing (`%LOCALAPPDATA%\Riot Games\Riot Client\Config\lockfile`)
//! - process detection for the Riot Client and the game (read-only, via sysinfo)
//! - discovery of local per-account VALORANT config folders
//!
//! Not implemented: talking to the local client API. That lives behind the
//! `RiotSettingsApi` trait so the rest of the app is already wired for it.
//! ValSync never touches game memory and never stores Riot credentials; the
//! lockfile password is held in memory only for the lifetime of a request.

use crate::error::{AppError, AppResult};
use crate::model::SettingsBundle;
use serde::Serialize;
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;
use sysinfo::{ProcessesToUpdate, System};

/// Parsed `name:pid:port:password:protocol` lockfile.
#[derive(Clone, PartialEq)]
pub struct Lockfile {
    pub name: String,
    pub pid: u32,
    pub port: u16,
    pub password: String,
    pub protocol: String,
}

impl fmt::Debug for Lockfile {
    // Never print the password, not even in debug logs.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Lockfile")
            .field("name", &self.name)
            .field("pid", &self.pid)
            .field("port", &self.port)
            .field("password", &"<redacted>")
            .field("protocol", &self.protocol)
            .finish()
    }
}

impl FromStr for Lockfile {
    type Err = AppError;

    fn from_str(s: &str) -> AppResult<Lockfile> {
        let bad = |why: &str| AppError::Riot(format!("malformed lockfile: {why}"));
        let parts: Vec<&str> = s.trim().split(':').collect();
        if parts.len() != 5 {
            return Err(bad("expected 5 fields"));
        }
        let pid = parts[1].parse().map_err(|_| bad("pid is not a number"))?;
        let port: u16 = parts[2].parse().map_err(|_| bad("port is not a number"))?;
        if port == 0 {
            return Err(bad("port is zero"));
        }
        let protocol = parts[4].to_ascii_lowercase();
        if protocol != "https" && protocol != "http" {
            return Err(bad("unknown protocol"));
        }
        if parts[3].is_empty() {
            return Err(bad("empty password"));
        }
        Ok(Lockfile { name: parts[0].to_string(), pid, port, password: parts[3].to_string(), protocol })
    }
}

impl Lockfile {
    pub fn base_url(&self) -> String {
        format!("{}://127.0.0.1:{}", self.protocol, self.port)
    }

    /// Value of the `Authorization` header for the local API (user `riot`).
    pub fn auth_header(&self) -> String {
        format!("Basic {}", base64(format!("riot:{}", self.password).as_bytes()))
    }
}

/// Minimal standard base64 encoder (RFC 4648, with padding).
pub fn base64(input: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(T[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

fn local_app_data() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(PathBuf::from)
}

pub fn lockfile_path() -> Option<PathBuf> {
    local_app_data().map(|p| p.join("Riot Games").join("Riot Client").join("Config").join("lockfile"))
}

/// Reads the lockfile if the Riot Client is running. The client keeps the file
/// open, so it is read with shared access through `std::fs::read_to_string`.
pub fn read_lockfile() -> AppResult<Option<Lockfile>> {
    let Some(path) = lockfile_path() else { return Ok(None) };
    match std::fs::read_to_string(&path) {
        Ok(s) => s.parse().map(Some),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// A local VALORANT config folder, one per account that ever logged in on this PC.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalProfile {
    pub folder: String,
    pub region: Option<String>,
    pub has_user_settings: bool,
    pub modified: u64,
}

/// Scans `%LOCALAPPDATA%\VALORANT\Saved\Config` for per-account folders
/// (`<uuid>-<region>`), newest first.
pub fn local_profiles() -> Vec<LocalProfile> {
    let Some(root) = local_app_data().map(|p| p.join("VALORANT").join("Saved").join("Config")) else { return Vec::new() };
    let Ok(entries) = std::fs::read_dir(root) else { return Vec::new() };
    let mut out: Vec<LocalProfile> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter_map(|e| {
            let folder = e.file_name().to_string_lossy().to_string();
            // Account folders look like 8-4-4-4-12 hex groups plus a region suffix.
            if folder.len() < 36 || folder.matches('-').count() < 4 {
                return None;
            }
            let region = folder.get(37..).filter(|r| !r.is_empty()).map(|r| r.to_uppercase());
            let settings = e.path().join("Windows").join("RiotUserSettings.ini");
            let modified = e
                .metadata()
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map_or(0, |d| d.as_secs());
            Some(LocalProfile { folder, region, has_user_settings: settings.exists(), modified })
        })
        .collect();
    out.sort_by(|a, b| b.modified.cmp(&a.modified));
    out
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RiotStatus {
    pub client_running: bool,
    pub game_running: bool,
    pub lockfile_found: bool,
    pub api_port: Option<u16>,
    pub local_profiles: usize,
}

const CLIENT_EXES: [&str; 2] = ["RiotClientServices.exe", "Riot Client.exe"];
const GAME_EXE: &str = "VALORANT-Win64-Shipping.exe";

/// Snapshot of what is running right now. Cheap enough to poll every few seconds.
pub fn status() -> RiotStatus {
    let mut sys = System::new();
    sys.refresh_processes(ProcessesToUpdate::All, true);
    let names: Vec<String> = sys.processes().values().map(|p| p.name().to_string_lossy().to_string()).collect();
    let running = |exe: &str| names.iter().any(|n| n.eq_ignore_ascii_case(exe));
    let lock = read_lockfile().ok().flatten();
    RiotStatus {
        client_running: CLIENT_EXES.iter().any(|e| running(e)),
        game_running: running(GAME_EXE),
        lockfile_found: lock.is_some(),
        api_port: lock.map(|l| l.port),
        local_profiles: local_profiles().len(),
    }
}

/// The contract the Riot client integration must fulfil.
pub trait RiotSettingsApi {
    /// Riot ID of the account currently signed in to the client.
    fn current_riot_id(&self) -> AppResult<String>;
    /// Pulls the signed-in account's settings into a bundle.
    fn fetch_settings(&self) -> AppResult<SettingsBundle>;
    /// Pushes a bundle to the signed-in account.
    fn push_settings(&self, bundle: &SettingsBundle) -> AppResult<()>;
}

/// Connection details for the local client API, built from the lockfile.
/// The auth header lives in memory only and is dropped with the struct.
pub struct LocalClientApi {
    endpoint: String,
    #[allow(dead_code)] // consumed by the HTTP calls in the TODOs below
    auth_header: String,
}

impl LocalClientApi {
    pub fn connect() -> AppResult<LocalClientApi> {
        let lockfile = read_lockfile()?.ok_or_else(|| AppError::Riot("Riot Client is not running".into()))?;
        log::info!("riot: lockfile found ({lockfile:?})");
        Ok(LocalClientApi { endpoint: lockfile.base_url(), auth_header: lockfile.auth_header() })
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientProbe {
    pub endpoint: Option<String>,
    pub riot_id: Option<String>,
    pub message: String,
}

/// Tries the full integration path and reports how far it got, for the status pill.
pub fn probe() -> ClientProbe {
    let api = match LocalClientApi::connect() {
        Ok(api) => api,
        Err(e) => return ClientProbe { endpoint: None, riot_id: None, message: e.to_string() },
    };
    let endpoint = Some(api.endpoint().to_string());
    match api.current_riot_id() {
        Ok(id) => ClientProbe { endpoint, riot_id: Some(id), message: "Connected".into() },
        Err(e) => ClientProbe { endpoint, riot_id: None, message: e.to_string() },
    }
}

impl RiotSettingsApi for LocalClientApi {
    // TODO(owner): GET the signed-in alias from the local client API using
    // `self.auth_header`; needs an HTTP client (not in the dependency set yet).
    fn current_riot_id(&self) -> AppResult<String> {
        Err(AppError::NotImplemented("local API: current account"))
    }

    // TODO(owner): fetch the player's settings blob, decode it and map it into
    // `SettingsBundle` categories (see `sample::bundle_for` for the key names).
    fn fetch_settings(&self) -> AppResult<SettingsBundle> {
        Err(AppError::NotImplemented("local API: fetch settings"))
    }

    // TODO(owner): encode the bundle back into the settings blob and PUT it.
    // The game must be closed; refuse while `status().game_running` is true.
    fn push_settings(&self, _bundle: &SettingsBundle) -> AppResult<()> {
        Err(AppError::NotImplemented("local API: push settings"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_lockfile() {
        let l: Lockfile = "Riot Client:18244:51537:k2Zr0Xq_example:https\r\n".parse().unwrap();
        assert_eq!((l.pid, l.port), (18244, 51537));
        assert_eq!(l.base_url(), "https://127.0.0.1:51537");
        assert!(l.auth_header().starts_with("Basic cmlvdDpr"));
    }

    #[test]
    fn rejects_malformed_lockfiles() {
        assert!("Riot Client:1:2:pw".parse::<Lockfile>().is_err());
        assert!("Riot Client:x:51537:pw:https".parse::<Lockfile>().is_err());
        assert!("Riot Client:1:0:pw:https".parse::<Lockfile>().is_err());
        assert!("Riot Client:1:51537::https".parse::<Lockfile>().is_err());
        assert!("Riot Client:1:51537:pw:ftp".parse::<Lockfile>().is_err());
    }

    #[test]
    fn debug_output_redacts_password() {
        let l: Lockfile = "Riot Client:1:51537:secret:https".parse().unwrap();
        let dbg = format!("{l:?}");
        assert!(!dbg.contains("secret") && dbg.contains("redacted"));
    }

    #[test]
    fn base64_matches_rfc_vectors() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
        assert_eq!(base64(b"riot:pw"), "cmlvdDpwdw==");
    }
}
