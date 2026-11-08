//! Cherry-pick diff between two settings bundles.
//!
//! `diff` walks both bundles key by key and classifies every setting.
//! `cherry_pick` then writes only the keys the user ticked in the diff view
//! from the source into a copy of the target, leaving everything else as-is.

use crate::error::{AppError, AppResult};
use crate::model::{Category, SettingValue, SettingsBundle};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ChangeKind {
    Same,
    Changed,
    /// Present in source only: applying adds it to the target.
    Added,
    /// Present in target only: applying removes it from the target.
    Removed,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffEntry {
    pub category: Category,
    pub key: String,
    pub source: Option<SettingValue>,
    pub target: Option<SettingValue>,
    pub kind: ChangeKind,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffReport {
    pub entries: Vec<DiffEntry>,
    pub changed: usize,
    pub added: usize,
    pub removed: usize,
    pub same: usize,
    /// Number of non-identical keys per category, for the segmented control badges.
    pub per_category: BTreeMap<Category, usize>,
}

impl DiffReport {
    pub fn pending(&self) -> usize {
        self.changed + self.added + self.removed
    }

    /// Every key that would change the target, i.e. the default selection.
    pub fn actionable(&self) -> Vec<KeyRef> {
        self.entries
            .iter()
            .filter(|e| e.kind != ChangeKind::Same)
            .map(|e| KeyRef { category: e.category, key: e.key.clone() })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct KeyRef {
    pub category: Category,
    pub key: String,
}

/// Computes the per-setting diff for the requested categories (all when empty).
pub fn diff(source: &SettingsBundle, target: &SettingsBundle, cats: &[Category]) -> DiffReport {
    let cats: Vec<Category> = if cats.is_empty() { Category::ALL.to_vec() } else { cats.to_vec() };
    let mut report = DiffReport::default();
    let empty = BTreeMap::new();
    for cat in cats {
        let s = source.categories.get(&cat).unwrap_or(&empty);
        let t = target.categories.get(&cat).unwrap_or(&empty);
        let keys: BTreeSet<&String> = s.keys().chain(t.keys()).collect();
        for key in keys {
            let (sv, tv) = (s.get(key), t.get(key));
            let kind = match (sv, tv) {
                (Some(a), Some(b)) if a == b => ChangeKind::Same,
                (Some(_), Some(_)) => ChangeKind::Changed,
                (Some(_), None) => ChangeKind::Added,
                (None, _) => ChangeKind::Removed,
            };
            match kind {
                ChangeKind::Same => report.same += 1,
                ChangeKind::Changed => report.changed += 1,
                ChangeKind::Added => report.added += 1,
                ChangeKind::Removed => report.removed += 1,
            }
            if kind != ChangeKind::Same {
                *report.per_category.entry(cat).or_default() += 1;
            }
            report.entries.push(DiffEntry { category: cat, key: key.clone(), source: sv.cloned(), target: tv.cloned(), kind });
        }
    }
    report
}

/// Applies only `selected` keys from `source` onto a copy of `target`.
/// Returns the new bundle and the number of keys that actually changed.
pub fn cherry_pick(source: &SettingsBundle, target: &SettingsBundle, selected: &[KeyRef]) -> AppResult<(SettingsBundle, usize)> {
    let mut out = target.clone();
    let mut changed = 0;
    let unique: BTreeSet<&KeyRef> = selected.iter().collect();
    for k in unique {
        match (source.get(k.category, &k.key), target.get(k.category, &k.key)) {
            (Some(sv), tv) => {
                if tv != Some(sv) {
                    out.set(k.category, k.key.clone(), sv.clone());
                    changed += 1;
                }
            }
            (None, Some(_)) => {
                out.remove(k.category, &k.key);
                changed += 1;
            }
            (None, None) => {
                return Err(AppError::NotFound(format!("{} / {}", k.category.label(), k.key)));
            }
        }
    }
    Ok((out, changed))
}

/// Whole-category copy used by the one-to-many transfer: the selected
/// categories of the target are replaced, all others are untouched.
pub fn apply_categories(source: &SettingsBundle, target: &SettingsBundle, cats: &[Category]) -> (SettingsBundle, usize) {
    let report = diff(source, target, cats);
    let mut out = target.clone();
    for cat in cats {
        match source.categories.get(cat) {
            Some(m) => {
                out.categories.insert(*cat, m.clone());
            }
            None => {
                out.categories.remove(cat);
            }
        }
    }
    (out, report.pending())
}

#[cfg(test)]
mod tests {
    use super::*;
    use SettingValue::*;

    fn bundles() -> (SettingsBundle, SettingsBundle) {
        let mut s = SettingsBundle::default();
        s.set(Category::Sensitivity, "MouseSensitivity", Float(0.32));
        s.set(Category::Sensitivity, "ScopedMultiplier", Float(1.0));
        s.set(Category::Keybinds, "Ability1", Text("Q".into()));
        s.set(Category::Keybinds, "Ping", Text("Mouse4".into()));
        s.set(Category::Video, "LimitFps", Int(240));
        let mut t = SettingsBundle::default();
        t.set(Category::Sensitivity, "MouseSensitivity", Float(0.45));
        t.set(Category::Sensitivity, "ScopedMultiplier", Float(1.0));
        t.set(Category::Keybinds, "Ability1", Text("Q".into()));
        t.set(Category::Keybinds, "Spray", Text("T".into()));
        t.set(Category::Hud, "ShowCorpses", Bool(true));
        (s, t)
    }

    #[test]
    fn classifies_every_key() {
        let (s, t) = bundles();
        let r = diff(&s, &t, &[]);
        assert_eq!((r.changed, r.added, r.removed, r.same), (1, 2, 2, 2));
        assert_eq!(r.pending(), 5);
        assert_eq!(r.per_category.get(&Category::Keybinds), Some(&2));
        let sens = r.entries.iter().find(|e| e.key == "MouseSensitivity").unwrap();
        assert_eq!(sens.kind, ChangeKind::Changed);
    }

    #[test]
    fn category_filter_limits_entries() {
        let (s, t) = bundles();
        let r = diff(&s, &t, &[Category::Sensitivity]);
        assert_eq!(r.entries.len(), 2);
        assert_eq!(r.pending(), 1);
    }

    #[test]
    fn cherry_pick_applies_only_selected() {
        let (s, t) = bundles();
        let pick = vec![
            KeyRef { category: Category::Sensitivity, key: "MouseSensitivity".into() },
            KeyRef { category: Category::Keybinds, key: "Spray".into() },
            KeyRef { category: Category::Keybinds, key: "Ability1".into() },
        ];
        let (out, n) = cherry_pick(&s, &t, &pick).unwrap();
        assert_eq!(n, 2, "Ability1 is identical and must not count");
        assert_eq!(out.get(Category::Sensitivity, "MouseSensitivity"), Some(&Float(0.32)));
        assert_eq!(out.get(Category::Keybinds, "Spray"), None);
        assert_eq!(out.get(Category::Keybinds, "Ping"), None, "unselected key stays absent");
        assert_eq!(out.get(Category::Hud, "ShowCorpses"), Some(&Bool(true)));
        assert!(diff(&s, &out, &[Category::Sensitivity]).pending() == 0);
    }

    #[test]
    fn cherry_pick_rejects_unknown_keys() {
        let (s, t) = bundles();
        let bad = [KeyRef { category: Category::Audio, key: "VoiceVolume".into() }];
        assert!(cherry_pick(&s, &t, &bad).is_err());
    }

    #[test]
    fn selecting_all_actionable_makes_bundles_equal() {
        let (s, t) = bundles();
        let r = diff(&s, &t, &[]);
        let (out, n) = cherry_pick(&s, &t, &r.actionable()).unwrap();
        assert_eq!(n, 5);
        assert_eq!(out, s);
    }

    #[test]
    fn category_copy_leaves_others_untouched() {
        let (s, t) = bundles();
        let (out, n) = apply_categories(&s, &t, &[Category::Keybinds, Category::Hud]);
        assert_eq!(n, 3);
        assert_eq!(out.count(Category::Keybinds), 2);
        assert_eq!(out.count(Category::Hud), 0);
        assert_eq!(out.get(Category::Sensitivity, "MouseSensitivity"), Some(&Float(0.45)));
    }
}
