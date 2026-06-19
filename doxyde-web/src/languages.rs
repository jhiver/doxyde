// Supported languages and text-direction logic.
// Ported from yatoo.travel frontend/src/lib/data/languages.ts — single source of
// truth for language labels (used by the switcher and <html lang/dir>).

/// A supported language: BCP-47 code, native label, and base text direction.
#[derive(Debug, Clone, Copy)]
pub struct LangInfo {
    pub code: &'static str,
    pub label: &'static str,
    pub dir: &'static str,
}

/// The full set of languages doxyde knows how to label. A site enables a subset
/// via the `i18n_enabled_lang` table; the canonical/source language is `en`.
pub const LANGUAGES: &[LangInfo] = &[
    LangInfo {
        code: "en",
        label: "English",
        dir: "ltr",
    },
    LangInfo {
        code: "fr",
        label: "Français",
        dir: "ltr",
    },
    LangInfo {
        code: "es",
        label: "Español",
        dir: "ltr",
    },
    LangInfo {
        code: "pt",
        label: "Português",
        dir: "ltr",
    },
    LangInfo {
        code: "ar",
        label: "العربية",
        dir: "rtl",
    },
    LangInfo {
        code: "sw",
        label: "Kiswahili",
        dir: "ltr",
    },
    LangInfo {
        code: "wo",
        label: "Wolof",
        dir: "ltr",
    },
    LangInfo {
        code: "ha",
        label: "Hausa",
        dir: "ltr",
    },
    LangInfo {
        code: "am",
        label: "አማርኛ",
        dir: "ltr",
    },
    LangInfo {
        code: "yo",
        label: "Yorùbá",
        dir: "ltr",
    },
    LangInfo {
        code: "ig",
        label: "Igbo",
        dir: "ltr",
    },
    LangInfo {
        code: "zu",
        label: "isiZulu",
        dir: "ltr",
    },
    LangInfo {
        code: "de",
        label: "Deutsch",
        dir: "ltr",
    },
    LangInfo {
        code: "it",
        label: "Italiano",
        dir: "ltr",
    },
    LangInfo {
        code: "nl",
        label: "Nederlands",
        dir: "ltr",
    },
    LangInfo {
        code: "zh",
        label: "中文",
        dir: "ltr",
    },
    LangInfo {
        code: "ja",
        label: "日本語",
        dir: "ltr",
    },
];

/// Right-to-left languages (broader than the labelled set, for future use).
const RTL_LANGUAGES: &[&str] = &["ar", "he", "fa", "ur"];

/// Text direction for a language code ("rtl" or "ltr").
pub fn get_direction(lang: &str) -> &'static str {
    if RTL_LANGUAGES.contains(&lang) {
        "rtl"
    } else {
        "ltr"
    }
}

/// Look up the native label for a language code (falls back to the code).
pub fn label_for(code: &str) -> &str {
    LANGUAGES
        .iter()
        .find(|l| l.code == code)
        .map(|l| l.label)
        .unwrap_or(code)
}

/// Whether a code is one doxyde knows how to label.
pub fn is_known(code: &str) -> bool {
    LANGUAGES.iter().any(|l| l.code == code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rtl_detection() {
        assert_eq!(get_direction("ar"), "rtl");
        assert_eq!(get_direction("he"), "rtl");
        assert_eq!(get_direction("en"), "ltr");
        assert_eq!(get_direction("fr"), "ltr");
    }

    #[test]
    fn labels() {
        assert_eq!(label_for("fr"), "Français");
        assert_eq!(label_for("xx"), "xx");
        assert!(is_known("en"));
        assert!(!is_known("xx"));
    }
}
