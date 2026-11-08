//! Domain model: accounts, setting values, settings bundles and snapshots.
//!
//! A `SettingsBundle` is the unit ValSync moves around. It is a two-level map
//! `Category -> key -> value`, serialized as plain JSON so snapshots stay
//! human-readable inside the vault folder.

use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

/// Setting groups as shown in the category checklist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Crosshair,
    Sensitivity,
    Keybinds,
    Video,
    Audio,
    Minimap,
    Hud,
}

impl Category {
    pub const ALL: [Category; 7] = [
        Category::Crosshair,
        Category::Sensitivity,
        Category::Keybinds,
        Category::Video,
        Category::Audio,
        Category::Minimap,
        Category::Hud,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Category::Crosshair => "Crosshair",
            Category::Sensitivity => "Sensitivity",
            Category::Keybinds => "Keybinds",
            Category::Video => "Video",
            Category::Audio => "Audio",
            Category::Minimap => "Minimap",
            Category::Hud => "HUD",
        }
    }

    /// Unit used in the checklist counter, e.g. "Crosshair · 3 profiles".
    pub fn unit(self, n: usize) -> &'static str {
        match (self, n) {
            (Category::Crosshair, 1) => "profile",
            (Category::Crosshair, _) => "profiles",
            (Category::Keybinds, 1) => "binding",
            (Category::Keybinds, _) => "bindings",
            (_, 1) => "setting",
            _ => "settings",
        }
    }
}

/// A single setting value. Tagged so the UI can render toggles, sliders and text.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "lowercase")]
pub enum SettingValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(String),
}

impl PartialEq for SettingValue {
    fn eq(&self, other: &Self) -> bool {
        use SettingValue::*;
        match (self, other) {
            (Bool(a), Bool(b)) => a == b,
            (Int(a), Int(b)) => a == b,
            (Float(a), Float(b)) => (a - b).abs() < 1e-6,
            (Int(a), Float(b)) | (Float(b), Int(a)) => (*a as f64 - b).abs() < 1e-6,
            (Text(a), Text(b)) => a == b,
            _ => false,
        }
    }
}

impl fmt::Display for SettingValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SettingValue::Bool(true) => f.write_str("On"),
            SettingValue::Bool(false) => f.write_str("Off"),
            SettingValue::Int(v) => write!(f, "{v}"),
            SettingValue::Float(v) => f.write_str(&trim_float(*v, 3)),
            SettingValue::Text(v) => f.write_str(v),
        }
    }
}

/// Formats a float with at most `decimals` places and no trailing zeros.
pub fn trim_float(v: f64, decimals: usize) -> String {
    let s = format!("{v:.decimals$}");
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    }
}

pub type CategoryMap = BTreeMap<String, SettingValue>;

/// All settings of one account, grouped by category.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SettingsBundle {
    pub categories: BTreeMap<Category, CategoryMap>,
}

impl SettingsBundle {
    pub fn get(&self, cat: Category, key: &str) -> Option<&SettingValue> {
        self.categories.get(&cat).and_then(|m| m.get(key))
    }

    pub fn set(&mut self, cat: Category, key: impl Into<String>, value: SettingValue) {
        self.categories.entry(cat).or_default().insert(key.into(), value);
    }

    pub fn remove(&mut self, cat: Category, key: &str) -> Option<SettingValue> {
        let map = self.categories.get_mut(&cat)?;
        let out = map.remove(key);
        if map.is_empty() {
            self.categories.remove(&cat);
        }
        out
    }

    pub fn count(&self, cat: Category) -> usize {
        self.categories.get(&cat).map_or(0, |m| m.len())
    }

    pub fn total(&self) -> usize {
        self.categories.values().map(|m| m.len()).sum()
    }
}

/// A Riot account known to ValSync. Credentials are never part of this model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: String,
    pub riot_id: String,
    pub region: String,
    pub rank: String,
    pub level: u32,
    pub is_main: bool,
    pub last_synced: Option<u64>,
}

impl Account {
    pub fn new(riot_id: &str, region: &str, rank: &str, level: u32, is_main: bool) -> AppResult<Self> {
        let (name, tag) = parse_riot_id(riot_id)?;
        Ok(Account {
            id: account_key(&name, &tag),
            riot_id: format!("{name}#{tag}"),
            region: region.to_uppercase(),
            rank: rank.to_string(),
            level,
            is_main,
            last_synced: None,
        })
    }

    /// First letter of the rank tier, shown in the round badge ("I" for Immortal).
    pub fn rank_initial(&self) -> char {
        self.rank.chars().find(|c| c.is_alphabetic()).map_or('U', |c| c.to_ascii_uppercase())
    }
}

/// Validates and splits a Riot ID. Name: 3-16 chars, tag: 3-5 alphanumerics.
pub fn parse_riot_id(s: &str) -> AppResult<(String, String)> {
    let bad = || AppError::InvalidRiotId(s.to_string());
    let (name, tag) = s.trim().rsplit_once('#').ok_or_else(bad)?;
    let name = name.trim();
    let name_len = name.chars().count();
    if !(3..=16).contains(&name_len) || name.chars().any(|c| c.is_control() || c == '#') {
        return Err(bad());
    }
    if !(3..=5).contains(&tag.len()) || !tag.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(bad());
    }
    Ok((name.to_string(), tag.to_uppercase()))
}

/// Stable, file-system safe key for an account ("night jar#EUW1" -> "night-jar-euw1").
pub fn account_key(name: &str, tag: &str) -> String {
    let mut out = String::new();
    for c in name.chars().chain(std::iter::once('-')).chain(tag.chars()) {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SnapshotKind {
    Manual,
    AutoBackup,
    Imported,
    Applied,
}

/// A versioned, immutable copy of an account's settings stored in the vault.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub id: String,
    pub account_id: String,
    pub created_at: u64,
    pub label: String,
    pub kind: SnapshotKind,
    pub bundle: SettingsBundle,
}

/// Lightweight listing entry for the backups timeline.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotSummary {
    pub id: String,
    pub account_id: String,
    pub created_at: u64,
    pub created_iso: String,
    pub label: String,
    pub kind: SnapshotKind,
    pub counts: BTreeMap<Category, usize>,
    pub total: usize,
    pub bytes: u64,
}

impl Snapshot {
    pub fn summary(&self, bytes: u64) -> SnapshotSummary {
        SnapshotSummary {
            id: self.id.clone(),
            account_id: self.account_id.clone(),
            created_at: self.created_at,
            created_iso: iso8601(self.created_at),
            label: self.label.clone(),
            kind: self.kind,
            counts: self.bundle.categories.iter().map(|(c, m)| (*c, m.len())).collect(),
            total: self.bundle.total(),
            bytes,
        }
    }
}

pub fn now_unix() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

/// Formats unix seconds as `YYYY-MM-DDTHH:MM:SSZ` without pulling in a date crate.
pub fn iso8601(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", rem / 3600, (rem % 3600) / 60, rem % 60)
}

/// Howard Hinnant's days-to-civil algorithm.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn riot_id_validation() {
        assert_eq!(parse_riot_id("Nightjar#euw1").unwrap(), ("Nightjar".into(), "EUW1".into()));
        assert!(parse_riot_id("ab#EUW").is_err());
        assert!(parse_riot_id("Nightjar").is_err());
        assert!(parse_riot_id("Nightjar#E!").is_err());
        assert_eq!(account_key("night jar", "EUW1"), "night-jar-euw1");
    }

    #[test]
    fn values_compare_and_display() {
        assert_eq!(SettingValue::Float(0.35), SettingValue::Float(0.350_000_1));
        assert_eq!(SettingValue::Int(2), SettingValue::Float(2.0));
        assert_ne!(SettingValue::Bool(true), SettingValue::Int(1));
        assert_eq!(SettingValue::Float(0.500).to_string(), "0.5");
        assert_eq!(SettingValue::Bool(false).to_string(), "Off");
    }

    #[test]
    fn iso_dates() {
        assert_eq!(iso8601(0), "1970-01-01T00:00:00Z");
        assert_eq!(iso8601(1_794_441_600), "2026-11-12T00:00:00Z");
    }

    #[test]
    fn bundle_serializes_with_lowercase_keys() {
        let mut b = SettingsBundle::default();
        b.set(Category::Hud, "ShowCorpses", SettingValue::Bool(false));
        let json = serde_json::to_string(&b).unwrap();
        assert!(json.contains("\"hud\""));
        let back: SettingsBundle = serde_json::from_str(&json).unwrap();
        assert_eq!(back, b);
    }
}
