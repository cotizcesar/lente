//! Translations into the 14 languages of the official app (da, de, en, es, fr, it, ja, ko, nl,
//! pt-BR, pt-PT, ru, sv, zh-Hant — the <Resource Language> list of its AppxManifest.xml).
//!
//! Keys are the Spanish texts used in the code; `i18n/strings.tsv` holds one row per key:
//! `es \t en \t de \t fr \t it \t pt-BR \t pt-PT \t nl \t da \t sv \t ru \t ja \t ko \t zh-Hant`.
//! A missing cell falls back to English, then to the key. `{}` marks where a value goes.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    Es,
    En,
    De,
    Fr,
    It,
    PtBr,
    PtPt,
    Nl,
    Da,
    Sv,
    Ru,
    Ja,
    Ko,
    ZhHant,
}

impl Lang {
    pub const ALL: [Lang; 14] = [
        Lang::Da, Lang::De, Lang::En, Lang::Es, Lang::Fr, Lang::It, Lang::Ja, Lang::Ko, Lang::Nl, Lang::PtBr, Lang::PtPt, Lang::Ru, Lang::Sv, Lang::ZhHant,
    ];
    pub fn code(self) -> &'static str {
        match self {
            Lang::Es => "es",
            Lang::En => "en",
            Lang::De => "de",
            Lang::Fr => "fr",
            Lang::It => "it",
            Lang::PtBr => "pt-BR",
            Lang::PtPt => "pt-PT",
            Lang::Nl => "nl",
            Lang::Da => "da",
            Lang::Sv => "sv",
            Lang::Ru => "ru",
            Lang::Ja => "ja",
            Lang::Ko => "ko",
            Lang::ZhHant => "zh-Hant",
        }
    }
    /// Name of the language in itself, for the language picker.
    pub fn native(self) -> &'static str {
        match self {
            Lang::Es => "Español",
            Lang::En => "English",
            Lang::De => "Deutsch",
            Lang::Fr => "Français",
            Lang::It => "Italiano",
            Lang::PtBr => "Português (Brasil)",
            Lang::PtPt => "Português (Portugal)",
            Lang::Nl => "Nederlands",
            Lang::Da => "Dansk",
            Lang::Sv => "Svenska",
            Lang::Ru => "Русский",
            Lang::Ja => "日本語",
            Lang::Ko => "한국어",
            Lang::ZhHant => "繁體中文",
        }
    }
    fn column(self) -> usize {
        match self {
            Lang::Es => 0,
            Lang::En => 1,
            Lang::De => 2,
            Lang::Fr => 3,
            Lang::It => 4,
            Lang::PtBr => 5,
            Lang::PtPt => 6,
            Lang::Nl => 7,
            Lang::Da => 8,
            Lang::Sv => 9,
            Lang::Ru => 10,
            Lang::Ja => 11,
            Lang::Ko => 12,
            Lang::ZhHant => 13,
        }
    }
    pub fn needs_cjk(self) -> bool {
        matches!(self, Lang::Ja | Lang::Ko | Lang::ZhHant)
    }
    pub fn from_code(code: &str) -> Option<Lang> {
        let c = code.to_ascii_lowercase().replace('_', "-");
        let c = c.split('.').next().unwrap_or("");
        Some(match c {
            x if x.starts_with("pt-pt") => Lang::PtPt,
            x if x.starts_with("pt") => Lang::PtBr,
            x if x.starts_with("zh") => Lang::ZhHant,
            x if x.starts_with("es") => Lang::Es,
            x if x.starts_with("en") => Lang::En,
            x if x.starts_with("de") => Lang::De,
            x if x.starts_with("fr") => Lang::Fr,
            x if x.starts_with("it") => Lang::It,
            x if x.starts_with("nl") => Lang::Nl,
            x if x.starts_with("da") => Lang::Da,
            x if x.starts_with("sv") => Lang::Sv,
            x if x.starts_with("ru") => Lang::Ru,
            x if x.starts_with("ja") => Lang::Ja,
            x if x.starts_with("ko") => Lang::Ko,
            _ => return None,
        })
    }
    /// From LENTE_LANG, then the saved choice, then LC_ALL / LC_MESSAGES / LANG; English otherwise.
    pub fn detect() -> Lang {
        if let Some(l) = std::env::var("LENTE_LANG").ok().and_then(|v| Lang::from_code(&v)) {
            return l;
        }
        if let Some(l) = saved_choice() {
            return l;
        }
        for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
            if let Some(l) = std::env::var(var).ok().filter(|v| !v.is_empty() && v != "C" && v != "POSIX").and_then(|v| Lang::from_code(&v)) {
                return l;
            }
        }
        Lang::En
    }
}

static CURRENT: AtomicU8 = AtomicU8::new(255);

pub fn lang() -> Lang {
    let v = CURRENT.load(Ordering::Relaxed);
    if v == 255 {
        let l = Lang::detect();
        CURRENT.store(l.column() as u8, Ordering::Relaxed);
        return l;
    }
    Lang::ALL.iter().copied().find(|l| l.column() as u8 == v).unwrap_or(Lang::En)
}

pub fn set_lang(l: Lang) {
    CURRENT.store(l.column() as u8, Ordering::Relaxed);
    if let Some(dir) = crate::camera::config_dir().to_str().map(str::to_string) {
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(format!("{dir}/idioma"), l.code());
    }
}

fn saved_choice() -> Option<Lang> {
    let p = crate::camera::config_dir().join("idioma");
    std::fs::read_to_string(p).ok().and_then(|s| Lang::from_code(s.trim()))
}

type Table = HashMap<&'static str, Vec<&'static str>>;

fn table() -> &'static Table {
    static T: OnceLock<Table> = OnceLock::new();
    T.get_or_init(|| {
        let mut m = HashMap::new();
        for line in include_str!("../i18n/strings.tsv").lines() {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let cols: Vec<&'static str> = line.split('\t').collect();
            m.insert(cols[0], cols);
        }
        m
    })
}

/// Translate a Spanish key into the current language.
pub fn tr(key: &str) -> String {
    let col = lang().column();
    if col == 0 {
        return key.to_string();
    }
    match table().get(key) {
        Some(cols) => cols
            .get(col)
            .filter(|s| !s.is_empty())
            .or_else(|| cols.get(1).filter(|s| !s.is_empty()))
            .map(|s| s.to_string())
            .unwrap_or_else(|| key.to_string()),
        None => key.to_string(),
    }
}

/// Translate and fill each `{}` in order.
pub fn trf(key: &str, args: &[&dyn std::fmt::Display]) -> String {
    let mut out = String::new();
    let mut rest = tr(key);
    for a in args {
        match rest.find("{}") {
            Some(i) => {
                out.push_str(&rest[..i]);
                out.push_str(&a.to_string());
                rest = rest[i + 2..].to_string();
            }
            None => break,
        }
    }
    out.push_str(&rest);
    out
}

pub fn tr_n(key: &str, n: impl std::fmt::Display) -> String {
    trf(key, &[&n])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_row_has_14_columns_and_placeholders_match() {
        for (k, cols) in table() {
            assert_eq!(cols.len(), 14, "row {k:?} has {} columns", cols.len());
            let n = k.matches("{}").count();
            for c in cols.iter().filter(|c| !c.is_empty()) {
                assert_eq!(c.matches("{}").count(), n, "placeholders differ in {k:?} -> {c:?}");
            }
        }
    }
    #[test]
    fn detects_locales() {
        assert_eq!(Lang::from_code("de_DE.UTF-8"), Some(Lang::De));
        assert_eq!(Lang::from_code("pt_PT.UTF-8"), Some(Lang::PtPt));
        assert_eq!(Lang::from_code("pt_BR"), Some(Lang::PtBr));
        assert_eq!(Lang::from_code("zh_TW.UTF-8"), Some(Lang::ZhHant));
    }
}
