//! Realistic demo data so every screen has content before the Riot client
//! integration exists. Key names double as the canonical setting identifiers
//! that `riot::RiotSettingsApi` implementations must map to.
//! TODO(owner): stop seeding once real accounts can be imported.

use crate::model::{Account, Category, SettingValue, SettingsBundle};
use SettingValue::{Bool, Float, Int, Text};

pub const KEYBINDS: [(&str, &str); 64] = [
    ("MoveForward", "W"), ("MoveBackward", "S"), ("StrafeLeft", "A"), ("StrafeRight", "D"),
    ("Walk", "Left Shift"), ("Crouch", "Left Ctrl"), ("Jump", "Space Bar"), ("JumpAlt", "Mouse Wheel Down"),
    ("Use", "F"), ("EquipPrimary", "1"), ("EquipSecondary", "2"), ("EquipMelee", "3"),
    ("EquipSpike", "4"), ("CycleNextWeapon", "Mouse Wheel Up"), ("CyclePrevWeapon", "Mouse Wheel Down"), ("Drop", "G"),
    ("Fire", "Left Mouse Button"), ("AltFire", "Right Mouse Button"), ("Reload", "R"), ("Inspect", "Y"),
    ("Ability1", "C"), ("Ability2", "Q"), ("Ability3", "E"), ("Ultimate", "X"),
    ("UseAbilityAlt", "Mouse Button 4"), ("Ping", "Middle Mouse Button"), ("PingAlt", "Z"), ("RadioCommands", ","),
    ("CommsWheel", "Mouse Button 5"), ("PushToTalkTeam", "V"), ("PushToTalkParty", "U"), ("Spray", "T"),
    ("SprayWheel", "Left Alt"), ("ToggleMap", "M"), ("MapZoomIn", "="), ("MapZoomOut", "-"),
    ("Scoreboard", "Tab"), ("BuyMenu", "B"), ("Chat", "Enter"), ("ChatTeam", "Left Shift + Enter"),
    ("ChatParty", "P"), ("ToggleCombatReport", "N"), ("ShowCrosshairCode", "F8"), ("ScreenshotHud", "F12"),
    ("EquipLastWeapon", "Q"), ("ScopeToggle", "Right Mouse Button"), ("MuteAll", "F7"), ("MuteTeam", "F6"),
    ("SpectatorNext", "Space Bar"), ("SpectatorPrev", "Left Ctrl"), ("ToggleFps", "F11"), ("ShowPing", "F10"),
    ("PlantDefuse", "4"), ("Interact", "F"), ("Emote", "J"), ("PetFlex", "K"),
    ("ShowAbilityInfo", "H"), ("ToggleWalk", "Caps Lock"), ("AltCrouch", "C"), ("AltFireAlt", "Mouse Button 5"),
    ("UseObjective", "F"), ("SwapUltPoints", "O"), ("MenuOpen", "Escape"), ("ToggleHud", "F9"),
];

const VIDEO: [(&str, &str); 18] = [
    ("DisplayMode", "Fullscreen"), ("Resolution", "1920x1080 16:9 (240Hz)"), ("AspectRatioMethod", "Fill"),
    ("LimitFpsOnBattery", "60"), ("LimitFpsInMenu", "60"), ("LimitFpsAlways", "Off"), ("NvidiaReflex", "On + Boost"),
    ("MaterialQuality", "Low"), ("TextureQuality", "Low"), ("DetailQuality", "Low"), ("UiQuality", "Low"),
    ("Vignette", "Off"), ("VSync", "Off"), ("AntiAliasing", "MSAA 4x"), ("AnisotropicFiltering", "4x"),
    ("ImproveClarity", "On"), ("Bloom", "Off"), ("Distortion", "Off"),
];

const AUDIO: [(&str, f64); 14] = [
    ("OverallVolume", 0.62), ("SoundEffects", 1.0), ("Music", 0.0), ("MenuMusic", 0.12), ("AgentFlavor", 0.4),
    ("VoiceOver", 0.85), ("VoiceChatVolume", 0.7), ("PartyVoice", 0.7), ("TeamVoice", 0.8), ("MicVolume", 1.15),
    ("MicSensitivity", 0.3), ("VoiceDucking", 0.5), ("AmbientEffects", 0.25), ("DistanceAttenuation", 1.0),
];

const HUD: [(&str, bool); 12] = [
    ("ShowCorpses", false), ("ShowBlood", false), ("ShowBulletTracers", true), ("ShowMuzzleFlash", true),
    ("ShowMoveErrorFeedback", true), ("ShowSpectatorCount", true), ("ShowMatchTimer", true), ("ShowKillFeed", true),
    ("ShowAbilityTooltips", false), ("ShowFriendlyOutlines", true), ("ShowObjectiveText", false), ("ShowSprayInPreview", false),
];

/// Crosshair profiles stored as their game codes.
pub const CROSSHAIRS: [(&str, &str); 3] = [
    ("Primary - cyan small", "0;P;c;5;h;0;0l;4;0o;2;0a;1;0f;0;1b;0"),
    ("Dot - violet", "0;P;c;8;u;8E7CFFFF;h;0;d;1;z;3;0b;0;1b;0"),
    ("Op - red plus", "0;P;c;7;t;2;o;1;0t;3;0l;5;0o;1;0a;1;1b;0;S;c;0"),
];

/// Five demo accounts: one main and four alts in different regions and ranks.
pub fn accounts() -> Vec<Account> {
    let rows = [
        ("Nightjar#EUW1", "EU", "Immortal 1", 287, true, Some(1_794_118_280)),
        ("nightjar smurf#7781", "EU", "Gold 3", 64, false, Some(1_793_909_100)),
        ("Kestrel#2290", "EU", "Platinum 1", 131, false, Some(1_793_520_340)),
        ("quietpeak#NA5", "NA", "Silver 2", 41, false, None),
        ("Halcyon#TR01", "EU", "Diamond 2", 176, false, Some(1_792_830_000)),
    ];
    rows.iter()
        .filter_map(|(id, region, rank, level, main, synced)| {
            let mut a = Account::new(id, region, rank, *level, *main).ok()?;
            a.last_synced = *synced;
            Some(a)
        })
        .collect()
}

/// Deterministic per-account variation: alts drift from the main's setup.
pub fn bundle_for(seed: u32) -> SettingsBundle {
    let mut b = SettingsBundle::default();
    let drift = |base: f64, step: f64, k: u32| base + step * ((seed.wrapping_mul(2_654_435_761).rotate_left(k)) % 5) as f64;

    for (i, (name, code)) in CROSSHAIRS.iter().enumerate() {
        let code = if seed > 0 && i == 0 { "0;P;c;1;0t;1;0l;3;0o;2;1b;0" } else { code };
        b.set(Category::Crosshair, *name, Text(code.to_string()));
    }

    b.set(Category::Sensitivity, "MouseSensitivity", Float(if seed == 0 { 0.32 } else { drift(0.35, 0.05, 3) }));
    b.set(Category::Sensitivity, "ScopedSensitivityMultiplier", Float(if seed == 0 { 1.0 } else { 0.95 }));
    b.set(Category::Sensitivity, "AdsSensitivityMultiplier", Float(1.0));
    b.set(Category::Sensitivity, "InvertMouse", Bool(false));
    b.set(Category::Sensitivity, "RawInputBuffer", Bool(seed != 2));
    b.set(Category::Sensitivity, "MouseDpi", Int(800));

    for (action, key) in KEYBINDS.iter() {
        let key = match (seed, *action) {
            (s, "Ability1") if s > 0 => "Q",
            (s, "Ability2") if s > 0 => "C",
            (s, "Walk") if s % 2 == 1 => "Caps Lock",
            (s, "Ping") if s > 0 => "Z",
            (s, "Inspect") if s == 3 => "I",
            _ => *key,
        };
        b.set(Category::Keybinds, *action, Text(key.to_string()));
    }

    for (k, v) in VIDEO {
        let v = match (seed, k) {
            (s, "AntiAliasing") if s > 0 => "FXAA",
            (s, "NvidiaReflex") if s > 1 => "On",
            (s, "Bloom") if s > 0 => "On",
            (s, "MaterialQuality") if s == 4 => "Medium",
            _ => v,
        };
        b.set(Category::Video, k, Text(v.to_string()));
    }

    for (k, v) in AUDIO {
        let v = if seed > 0 && k == "Music" { 0.35 } else if seed > 0 && k == "OverallVolume" { 0.8 } else { v };
        b.set(Category::Audio, k, Float(v));
    }

    b.set(Category::Minimap, "Rotate", Bool(seed == 0));
    b.set(Category::Minimap, "FixedOrientation", Text(if seed == 0 { "Based on Side" } else { "Always the Same" }.into()));
    b.set(Category::Minimap, "KeepPlayerCentered", Bool(seed != 0));
    b.set(Category::Minimap, "Size", Float(if seed == 0 { 1.1 } else { 1.0 }));
    b.set(Category::Minimap, "Zoom", Float(if seed == 0 { 0.9 } else { 0.85 }));
    b.set(Category::Minimap, "VisionCones", Bool(true));
    b.set(Category::Minimap, "ShowRegionNames", Text("Always".into()));

    for (k, v) in HUD {
        b.set(Category::Hud, k, Bool(if seed > 0 && (k == "ShowCorpses" || k == "ShowBlood") { true } else { v }));
    }
    b
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::diff;

    #[test]
    fn counts_match_the_checklist() {
        let b = bundle_for(0);
        let counts: Vec<usize> = Category::ALL.iter().map(|c| b.count(*c)).collect();
        assert_eq!(counts, vec![3, 6, 64, 18, 14, 7, 12]);
    }

    #[test]
    fn crosshair_samples_parse() {
        for (_, code) in CROSSHAIRS {
            assert_eq!(crate::crosshair::normalize(code).unwrap(), code);
        }
    }

    #[test]
    fn alts_differ_from_main() {
        let r = diff(&bundle_for(0), &bundle_for(2), &[]);
        assert!(r.changed > 10, "demo diff should be interesting, got {}", r.changed);
        assert_eq!(accounts().iter().filter(|a| a.is_main).count(), 1);
    }
}
