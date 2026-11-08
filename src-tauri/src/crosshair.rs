//! Valorant crosshair profile codes.
//!
//! A code is a `;`-separated token stream: a version number, then optional
//! global `key;value` pairs, then sections introduced by a single uppercase
//! letter (`P` primary, `A` aim-down-sights, `S` sniper scope), each followed
//! by `key;value` pairs. Only values that differ from the game defaults are
//! written, so `0;P;c;5;h;0;0l;4;0o;2;0a;1;0f;0;1b;0` reads as: cyan, no
//! outlines, inner lines length 4 / offset 2 / opacity 1 without firing error,
//! outer lines hidden.
//!
//! Unknown keys are preserved verbatim so a round trip never loses data.

use crate::error::{AppError, AppResult};
use crate::model::trim_float;
use serde::{Deserialize, Serialize};

/// Preset colors in the order the game lists them; index 8 means custom hex.
pub const PALETTE: [(&str, &str); 8] = [
    ("White", "#FFFFFF"),
    ("Green", "#00FF00"),
    ("Yellow Green", "#7FFF00"),
    ("Green Yellow", "#DFFF00"),
    ("Yellow", "#FFFF00"),
    ("Cyan", "#00FFFF"),
    ("Pink", "#FF00FF"),
    ("Red", "#FF0000"),
];
pub const CUSTOM_COLOR: u8 = 8;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LineSet {
    pub show: bool,
    pub thickness: f32,
    pub length: f32,
    pub offset: f32,
    pub opacity: f32,
    pub movement_error: bool,
    pub firing_error: bool,
}

impl LineSet {
    pub const INNER: LineSet = LineSet {
        show: true,
        thickness: 2.0,
        length: 6.0,
        offset: 3.0,
        opacity: 0.8,
        movement_error: false,
        firing_error: true,
    };
    pub const OUTER: LineSet = LineSet {
        show: true,
        thickness: 2.0,
        length: 2.0,
        offset: 10.0,
        opacity: 0.35,
        movement_error: true,
        firing_error: true,
    };
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub color: u8,
    pub custom_color: Option<String>,
    pub outlines: bool,
    pub outline_thickness: f32,
    pub outline_opacity: f32,
    pub center_dot: bool,
    pub dot_thickness: f32,
    pub dot_opacity: f32,
    pub fade_with_firing: bool,
    pub inner: LineSet,
    pub outer: LineSet,
    /// Unknown `key;value` pairs of this section, kept in original order.
    pub extra: Vec<(String, String)>,
}

impl Default for Profile {
    fn default() -> Self {
        Profile {
            color: 0,
            custom_color: None,
            outlines: true,
            outline_thickness: 1.0,
            outline_opacity: 0.5,
            center_dot: false,
            dot_thickness: 2.0,
            dot_opacity: 1.0,
            fade_with_firing: false,
            inner: LineSet::INNER,
            outer: LineSet::OUTER,
            extra: Vec::new(),
        }
    }
}

impl Profile {
    /// Resolved display color as `#RRGGBB`.
    pub fn hex(&self) -> String {
        match (&self.custom_color, self.color) {
            (Some(c), CUSTOM_COLOR) => format!("#{}", &c[..6.min(c.len())]),
            _ => PALETTE.get(self.color as usize).map_or("#FFFFFF", |p| p.1).to_string(),
        }
    }

    pub fn color_name(&self) -> String {
        match self.color {
            CUSTOM_COLOR => format!("Custom {}", self.hex()),
            i => PALETTE.get(i as usize).map_or("White", |p| p.0).to_string(),
        }
    }

    /// One-line human description for profile lists.
    pub fn describe(&self) -> String {
        let mut parts = vec![self.color_name()];
        if self.center_dot {
            parts.push(format!("dot {}", trim_float(self.dot_thickness as f64, 2)));
        }
        let lines = |n: &str, l: &LineSet| {
            if l.show {
                format!("{n} {}/{}/{}", fmt(l.length), fmt(l.thickness), fmt(l.offset))
            } else {
                format!("{n} off")
            }
        };
        parts.push(lines("inner", &self.inner));
        parts.push(lines("outer", &self.outer));
        if !self.outlines {
            parts.push("no outlines".into());
        }
        parts.join(" \u{00B7} ")
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrosshairCode {
    pub version: u32,
    /// Global pairs that precede the first section (e.g. `s;1` advanced options).
    pub general: Vec<(String, String)>,
    pub primary: Profile,
    pub ads: Option<Profile>,
    /// Sniper scope section kept as raw pairs; the game exposes few options there.
    pub sniper: Vec<(String, String)>,
}

impl Default for CrosshairCode {
    fn default() -> Self {
        CrosshairCode { version: 0, general: Vec::new(), primary: Profile::default(), ads: None, sniper: Vec::new() }
    }
}

fn fmt(v: f32) -> String {
    trim_float(v as f64, 3)
}

fn num(key: &str, v: &str) -> AppResult<f32> {
    let n: f32 = v.parse().map_err(|_| AppError::Crosshair(format!("value \"{v}\" for \"{key}\" is not a number")))?;
    if !n.is_finite() || n < 0.0 {
        return Err(AppError::Crosshair(format!("value \"{v}\" for \"{key}\" is out of range")));
    }
    Ok(n)
}

fn flag(key: &str, v: &str) -> AppResult<bool> {
    match v {
        "0" => Ok(false),
        "1" => Ok(true),
        _ => Err(AppError::Crosshair(format!("value \"{v}\" for \"{key}\" must be 0 or 1"))),
    }
}

fn apply_lines(l: &mut LineSet, k: &str, key: &str, v: &str) -> AppResult<bool> {
    match k {
        "b" => l.show = flag(key, v)?,
        "t" => l.thickness = num(key, v)?,
        "l" => l.length = num(key, v)?,
        "o" => l.offset = num(key, v)?,
        "a" => l.opacity = num(key, v)?.min(1.0),
        "m" => l.movement_error = flag(key, v)?,
        "f" => l.firing_error = flag(key, v)?,
        _ => return Ok(false),
    }
    Ok(true)
}

fn apply(p: &mut Profile, key: &str, v: &str) -> AppResult<()> {
    let known = match key {
        "c" => {
            let c = num(key, v)? as u8;
            if c > CUSTOM_COLOR {
                return Err(AppError::Crosshair(format!("color index {c} out of range")));
            }
            p.color = c;
            true
        }
        "u" => {
            if v.len() != 8 || !v.chars().all(|c| c.is_ascii_hexdigit()) {
                return Err(AppError::Crosshair(format!("custom color \"{v}\" must be RRGGBBAA")));
            }
            p.custom_color = Some(v.to_ascii_uppercase());
            true
        }
        "h" => { p.outlines = flag(key, v)?; true }
        "t" => { p.outline_thickness = num(key, v)?; true }
        "o" => { p.outline_opacity = num(key, v)?.min(1.0); true }
        "d" => { p.center_dot = flag(key, v)?; true }
        "z" => { p.dot_thickness = num(key, v)?; true }
        "a" => { p.dot_opacity = num(key, v)?.min(1.0); true }
        "f" => { p.fade_with_firing = flag(key, v)?; true }
        _ if key.len() == 2 && key.starts_with('0') => apply_lines(&mut p.inner, &key[1..], key, v)?,
        _ if key.len() == 2 && key.starts_with('1') => apply_lines(&mut p.outer, &key[1..], key, v)?,
        _ => false,
    };
    if !known {
        p.extra.push((key.to_string(), v.to_string()));
    }
    Ok(())
}

fn emit_lines(out: &mut Vec<(String, String)>, prefix: char, l: &LineSet, d: &LineSet) {
    let mut push = |k: char, v: String| out.push((format!("{prefix}{k}"), v));
    if l.show != d.show { push('b', (l.show as u8).to_string()); }
    if l.thickness != d.thickness { push('t', fmt(l.thickness)); }
    if l.length != d.length { push('l', fmt(l.length)); }
    if l.offset != d.offset { push('o', fmt(l.offset)); }
    if l.opacity != d.opacity { push('a', fmt(l.opacity)); }
    if l.movement_error != d.movement_error { push('m', (l.movement_error as u8).to_string()); }
    if l.firing_error != d.firing_error { push('f', (l.firing_error as u8).to_string()); }
}

/// Pairs that differ from the defaults, in the canonical order the game writes them.
fn emit(p: &Profile) -> Vec<(String, String)> {
    let d = Profile::default();
    let mut out: Vec<(String, String)> = Vec::new();
    let mut push = |k: &str, v: String| out.push((k.to_string(), v));
    if p.color != d.color { push("c", p.color.to_string()); }
    if let (CUSTOM_COLOR, Some(u)) = (p.color, &p.custom_color) { push("u", u.clone()); }
    if p.outlines != d.outlines { push("h", (p.outlines as u8).to_string()); }
    if p.outline_thickness != d.outline_thickness { push("t", fmt(p.outline_thickness)); }
    if p.outline_opacity != d.outline_opacity { push("o", fmt(p.outline_opacity)); }
    if p.center_dot != d.center_dot { push("d", (p.center_dot as u8).to_string()); }
    if p.dot_thickness != d.dot_thickness { push("z", fmt(p.dot_thickness)); }
    if p.dot_opacity != d.dot_opacity { push("a", fmt(p.dot_opacity)); }
    if p.fade_with_firing != d.fade_with_firing { push("f", (p.fade_with_firing as u8).to_string()); }
    emit_lines(&mut out, '0', &p.inner, &d.inner);
    emit_lines(&mut out, '1', &p.outer, &d.outer);
    out.extend(p.extra.iter().cloned());
    out
}

#[derive(Clone, Copy, PartialEq)]
enum Section {
    General,
    Primary,
    Ads,
    Sniper,
}

pub fn parse(code: &str) -> AppResult<CrosshairCode> {
    let code = code.trim().trim_end_matches(';');
    if code.is_empty() {
        return Err(AppError::Crosshair("code is empty".into()));
    }
    let mut tokens = code.split(';').map(str::trim);
    let version = tokens
        .next()
        .and_then(|t| t.parse::<u32>().ok())
        .ok_or_else(|| AppError::Crosshair("code must start with a version number".into()))?;
    let mut out = CrosshairCode { version, ..Default::default() };
    let mut section = Section::General;
    while let Some(tok) = tokens.next() {
        let next = match tok {
            "P" => Some(Section::Primary),
            "A" => Some(Section::Ads),
            "S" => Some(Section::Sniper),
            _ => None,
        };
        if let Some(s) = next {
            if s == Section::Ads {
                out.ads.get_or_insert_with(Profile::default);
            }
            section = s;
            continue;
        }
        if tok.is_empty() {
            return Err(AppError::Crosshair("empty token (double ';')".into()));
        }
        let value = tokens.next().ok_or_else(|| AppError::Crosshair(format!("key \"{tok}\" has no value")))?;
        match section {
            Section::General => out.general.push((tok.to_string(), value.to_string())),
            Section::Primary => apply(&mut out.primary, tok, value)?,
            Section::Ads => apply(out.ads.get_or_insert_with(Profile::default), tok, value)?,
            Section::Sniper => out.sniper.push((tok.to_string(), value.to_string())),
        }
    }
    Ok(out)
}

fn push_pairs(list: &[(String, String)], parts: &mut Vec<String>) {
    for (k, v) in list {
        parts.push(k.clone());
        parts.push(v.clone());
    }
}

pub fn encode(c: &CrosshairCode) -> String {
    let mut parts = vec![c.version.to_string()];
    let pairs = push_pairs;
    pairs(&c.general, &mut parts);
    parts.push("P".into());
    pairs(&emit(&c.primary), &mut parts);
    if let Some(ads) = &c.ads {
        // ADS values are stored relative to the game defaults, like the primary profile.
        parts.push("A".into());
        pairs(&emit(ads), &mut parts);
    }
    if !c.sniper.is_empty() {
        parts.push("S".into());
        pairs(&c.sniper, &mut parts);
    }
    parts.join(";")
}

/// Normalizes a pasted code (whitespace, trailing `;`, key order) into canonical form.
pub fn normalize(code: &str) -> AppResult<String> {
    parse(code).map(|c| encode(&c))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "0;P;c;5;h;0;0l;4;0o;2;0a;1;0f;0;1b;0";

    #[test]
    fn parses_sample_code() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.primary.color, 5);
        assert_eq!(c.primary.hex(), "#00FFFF");
        assert!(!c.primary.outlines);
        assert_eq!(c.primary.inner.length, 4.0);
        assert_eq!(c.primary.inner.offset, 2.0);
        assert_eq!(c.primary.inner.opacity, 1.0);
        assert!(!c.primary.inner.firing_error);
        assert!(!c.primary.outer.show);
        assert!(c.ads.is_none());
    }

    #[test]
    fn round_trips_exactly() {
        assert_eq!(encode(&parse(SAMPLE).unwrap()), SAMPLE);
        let advanced = "0;s;1;P;c;8;u;FF4655FF;h;0;d;1;z;3;0b;0;1b;0;A;c;1;S;c;0";
        assert_eq!(encode(&parse(advanced).unwrap()), advanced);
    }

    #[test]
    fn defaults_encode_minimal() {
        assert_eq!(encode(&CrosshairCode::default()), "0;P");
        assert_eq!(normalize(" 0;P;h;1; ").unwrap(), "0;P");
    }

    #[test]
    fn custom_color_and_floats() {
        let c = parse("0;P;c;8;u;8E7CFFFF;0t;1.5;0a;0.650").unwrap();
        assert_eq!(c.primary.hex(), "#8E7CFF");
        assert_eq!(c.primary.inner.thickness, 1.5);
        assert_eq!(encode(&c), "0;P;c;8;u;8E7CFFFF;0t;1.5;0a;0.65");
    }

    #[test]
    fn unknown_keys_survive() {
        let c = parse("0;P;c;1;0e;0.5;9q;2").unwrap();
        assert_eq!(c.primary.extra, vec![("0e".into(), "0.5".into()), ("9q".into(), "2".into())]);
        assert_eq!(encode(&c), "0;P;c;1;0e;0.5;9q;2");
    }

    #[test]
    fn rejects_bad_codes() {
        assert!(parse("").is_err());
        assert!(parse("P;c;1").is_err());
        assert!(parse("0;P;c").is_err());
        assert!(parse("0;P;c;12").is_err());
        assert!(parse("0;P;h;yes").is_err());
        assert!(parse("0;P;u;FF46").is_err());
    }

    #[test]
    fn describes_profile() {
        let c = parse(SAMPLE).unwrap();
        assert_eq!(c.primary.describe(), "Cyan \u{00B7} inner 4/2/2 \u{00B7} outer off \u{00B7} no outlines");
    }
}
