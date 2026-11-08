//! Settings vault: local, versioned snapshots per account.
//!
//! Layout under `%APPDATA%\ValSync`:
//! ```text
//! ValSync\
//!   config.json                 vault options (retention, auto-backup)
//!   accounts.json               known accounts (Riot ID, region, rank) - no credentials
//!   vault\<account-key>\<unix>-<id>.json   one immutable snapshot per file
//! ```
//! Writes go to a `.tmp` file first and are renamed into place, so a crash
//! never leaves a half-written snapshot behind.

use crate::error::{AppError, AppResult};
use crate::model::{now_unix, Account, SettingsBundle, Snapshot, SnapshotKind, SnapshotSummary};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct VaultConfig {
    /// Snapshot the target before every overwrite.
    pub auto_backup: bool,
    /// How many automatic backups to keep per account (manual ones are never pruned).
    pub keep_auto_backups: usize,
    /// ValSync never stores Riot credentials; the flag exists so the UI can state it.
    pub store_credentials: bool,
}

impl Default for VaultConfig {
    fn default() -> Self {
        VaultConfig { auto_backup: true, keep_auto_backups: 20, store_credentials: false }
    }
}

pub struct Vault {
    root: PathBuf,
}

static COUNTER: AtomicU32 = AtomicU32::new(0);

/// Short unique id: seconds + nanos + process-local counter, base 36.
pub fn new_id() -> String {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    let n = (now.as_secs() as u128) << 40 | (now.subsec_nanos() as u128) << 8 | (COUNTER.fetch_add(1, Ordering::Relaxed) & 0xFF) as u128;
    to_base36(n)
}

fn to_base36(mut n: u128) -> String {
    const DIGITS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut out = Vec::new();
    while n > 0 {
        out.push(DIGITS[(n % 36) as usize]);
        n /= 36;
    }
    out.reverse();
    String::from_utf8(out).unwrap_or_default()
}

fn safe_component(s: &str) -> String {
    s.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect()
}

impl Vault {
    /// `%APPDATA%\ValSync`, falling back to the working directory.
    pub fn default_root() -> PathBuf {
        std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(".")).join("ValSync")
    }

    pub fn open(root: impl Into<PathBuf>) -> AppResult<Vault> {
        let root = root.into();
        fs::create_dir_all(root.join("vault"))?;
        Ok(Vault { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn account_dir(&self, account_id: &str) -> PathBuf {
        self.root.join("vault").join(safe_component(account_id))
    }

    fn write_json<T: Serialize>(path: &Path, value: &T) -> AppResult<u64> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let bytes = serde_json::to_vec_pretty(value)?;
        let tmp = path.with_extension("tmp");
        fs::write(&tmp, &bytes)?;
        fs::rename(&tmp, path)?;
        Ok(bytes.len() as u64)
    }

    fn read_json<T: DeserializeOwned>(path: &Path) -> AppResult<T> {
        Ok(serde_json::from_slice(&fs::read(path)?)?)
    }

    pub fn load_config(&self) -> VaultConfig {
        Self::read_json(&self.root.join("config.json")).unwrap_or_default()
    }

    pub fn save_config(&self, cfg: &VaultConfig) -> AppResult<()> {
        Self::write_json(&self.root.join("config.json"), cfg).map(|_| ())
    }

    pub fn load_accounts(&self) -> AppResult<Vec<Account>> {
        let path = self.root.join("accounts.json");
        if !path.exists() {
            return Ok(Vec::new());
        }
        Self::read_json(&path)
    }

    pub fn save_accounts(&self, accounts: &[Account]) -> AppResult<()> {
        Self::write_json(&self.root.join("accounts.json"), &accounts).map(|_| ())
    }

    /// Creates and persists a new snapshot.
    pub fn snapshot(&self, account_id: &str, label: &str, kind: SnapshotKind, bundle: SettingsBundle) -> AppResult<SnapshotSummary> {
        let snap = Snapshot {
            id: new_id(),
            account_id: account_id.to_string(),
            created_at: now_unix(),
            label: label.to_string(),
            kind,
            bundle,
        };
        self.save(&snap)
    }

    pub fn save(&self, snap: &Snapshot) -> AppResult<SnapshotSummary> {
        let file = self.account_dir(&snap.account_id).join(format!("{:010}-{}.json", snap.created_at, safe_component(&snap.id)));
        let bytes = Self::write_json(&file, snap)?;
        log::info!("vault: saved snapshot {} for {} ({} bytes)", snap.id, snap.account_id, bytes);
        Ok(snap.summary(bytes))
    }

    fn snapshot_files(&self, account_id: Option<&str>) -> AppResult<Vec<PathBuf>> {
        let dirs: Vec<PathBuf> = match account_id {
            Some(a) => vec![self.account_dir(a)],
            None => fs::read_dir(self.root.join("vault"))?.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.is_dir()).collect(),
        };
        let mut files = Vec::new();
        for dir in dirs.iter().filter(|d| d.is_dir()) {
            for entry in fs::read_dir(dir)?.filter_map(|e| e.ok()) {
                let p = entry.path();
                if p.extension().is_some_and(|x| x == "json") {
                    files.push(p);
                }
            }
        }
        Ok(files)
    }

    /// Newest first. Corrupt files are skipped with a warning instead of failing the list.
    pub fn list(&self, account_id: Option<&str>) -> AppResult<Vec<SnapshotSummary>> {
        let mut out = Vec::new();
        for path in self.snapshot_files(account_id)? {
            match Self::read_json::<Snapshot>(&path) {
                Ok(s) => out.push(s.summary(fs::metadata(&path).map_or(0, |m| m.len()))),
                Err(e) => log::warn!("vault: skipping {}: {e}", path.display()),
            }
        }
        out.sort_by(|a, b| b.created_at.cmp(&a.created_at).then_with(|| b.id.cmp(&a.id)));
        Ok(out)
    }

    fn find_file(&self, id: &str) -> AppResult<PathBuf> {
        let suffix = format!("-{}.json", safe_component(id));
        self.snapshot_files(None)?
            .into_iter()
            .find(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.ends_with(&suffix)))
            .ok_or_else(|| AppError::NotFound(format!("snapshot {id}")))
    }

    pub fn load(&self, id: &str) -> AppResult<Snapshot> {
        Self::read_json(&self.find_file(id)?)
    }

    pub fn latest(&self, account_id: &str) -> AppResult<Option<Snapshot>> {
        match self.list(Some(account_id))?.first() {
            Some(s) => self.load(&s.id).map(Some),
            None => Ok(None),
        }
    }

    /// Removes automatic backups beyond `keep`, newest kept. Returns how many were deleted.
    pub fn prune(&self, account_id: &str, keep: usize) -> AppResult<usize> {
        let autos: Vec<SnapshotSummary> = self.list(Some(account_id))?.into_iter().filter(|s| s.kind == SnapshotKind::AutoBackup).collect();
        let mut removed = 0;
        for s in autos.iter().skip(keep) {
            fs::remove_file(self.find_file(&s.id)?)?;
            removed += 1;
        }
        Ok(removed)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::model::{Category, SettingValue};

    pub fn temp_vault(tag: &str) -> Vault {
        let dir = std::env::temp_dir().join(format!("valsync-test-{tag}-{}", new_id()));
        Vault::open(dir).unwrap()
    }

    fn bundle(sens: f64) -> SettingsBundle {
        let mut b = SettingsBundle::default();
        b.set(Category::Sensitivity, "MouseSensitivity", SettingValue::Float(sens));
        b
    }

    #[test]
    fn ids_are_unique() {
        let a = new_id();
        let b = new_id();
        assert_ne!(a, b);
        assert!(a.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn save_list_load_roundtrip() {
        let v = temp_vault("roundtrip");
        let s1 = v.snapshot("nightjar-euw1", "First", SnapshotKind::Manual, bundle(0.3)).unwrap();
        let s2 = v.snapshot("nightjar-euw1", "Second", SnapshotKind::Manual, bundle(0.4)).unwrap();
        v.snapshot("kestrel-2290", "Other", SnapshotKind::Imported, bundle(0.5)).unwrap();
        assert_eq!(v.list(Some("nightjar-euw1")).unwrap().len(), 2);
        assert_eq!(v.list(None).unwrap().len(), 3);
        assert_eq!(v.load(&s1.id).unwrap().label, "First");
        let latest = v.latest("nightjar-euw1").unwrap().unwrap();
        assert_eq!(latest.id, s2.id);
        fs::remove_dir_all(v.root()).ok();
    }

    #[test]
    fn prune_keeps_manual_snapshots() {
        let v = temp_vault("prune");
        v.snapshot("a-0001", "Manual", SnapshotKind::Manual, bundle(0.1)).unwrap();
        for i in 0..5 {
            v.snapshot("a-0001", &format!("Auto {i}"), SnapshotKind::AutoBackup, bundle(0.2)).unwrap();
        }
        assert_eq!(v.prune("a-0001", 2).unwrap(), 3);
        let left = v.list(Some("a-0001")).unwrap();
        assert_eq!(left.len(), 3);
        assert!(left.iter().any(|s| s.kind == SnapshotKind::Manual));
        fs::remove_dir_all(v.root()).ok();
    }

    #[test]
    fn config_defaults_when_missing() {
        let v = temp_vault("config");
        let cfg = v.load_config();
        assert!(cfg.auto_backup && !cfg.store_credentials);
        v.save_config(&VaultConfig { keep_auto_backups: 5, ..cfg }).unwrap();
        assert_eq!(v.load_config().keep_auto_backups, 5);
        fs::remove_dir_all(v.root()).ok();
    }
}
