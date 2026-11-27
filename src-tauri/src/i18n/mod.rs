//! The language the app speaks: the system's, read once at launch. Every user-facing string and
//! every prompt lives in a `Strings` table per language, so a missing translation fails to compile.
mod en;
mod ja;
mod zh_hans;
mod zh_hant;

use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    ZhHans,
    ZhHant,
    EnUs,
    EnAu,
    Ja,
}

impl Lang {
    pub fn tag(self) -> &'static str {
        match self {
            Lang::ZhHans => "zh-Hans",
            Lang::ZhHant => "zh-Hant",
            Lang::EnUs => "en-US",
            Lang::EnAu => "en-AU",
            Lang::Ja => "ja",
        }
    }

    /// Map a BCP 47 tag ("zh-Hans-CN", "en_GB", "ja-JP") to a supported language.
    pub fn from_tag(tag: &str) -> Option<Lang> {
        let tag = tag.trim().replace('_', "-").to_lowercase();
        let mut parts = tag.split('-');
        let lang = parts.next()?;
        let rest: Vec<&str> = parts.collect();
        let has = |x: &str| rest.contains(&x);
        match lang {
            "zh" if has("hant") || (!has("hans") && (has("tw") || has("hk") || has("mo"))) => Some(Lang::ZhHant),
            "zh" => Some(Lang::ZhHans),
            "yue" => Some(Lang::ZhHant),
            "en" if ["au", "nz", "gb", "ie"].iter().any(|r| has(r)) => Some(Lang::EnAu),
            "en" => Some(Lang::EnUs),
            "ja" => Some(Lang::Ja),
            _ => None,
        }
    }
}

static LANG: OnceLock<Lang> = OnceLock::new();

/// `POMO_LANG` (a dev aid) first, then the system's preferred languages in order; English otherwise.
fn detect() -> Lang {
    if let Some(l) = std::env::var("POMO_LANG").ok().as_deref().and_then(Lang::from_tag) {
        return l;
    }
    sys_locale::get_locales().find_map(|t| Lang::from_tag(&t)).unwrap_or(Lang::EnUs)
}

/// Settle the language for this run. Call once, before anything shows text.
pub fn init() -> Lang {
    *LANG.get_or_init(detect)
}

/// The language of this run. Before `init` (and in tests) it is Simplified Chinese, so tests don't
/// depend on the machine they run on.
pub fn lang() -> Lang {
    LANG.get().copied().unwrap_or(Lang::ZhHans)
}

pub fn strings(l: Lang) -> &'static Strings {
    match l {
        Lang::ZhHans => &zh_hans::S,
        Lang::ZhHant => &zh_hant::S,
        Lang::EnUs => &en::US,
        Lang::EnAu => &en::AU,
        Lang::Ja => &ja::S,
    }
}

/// The strings of this run's language.
pub fn tr() -> &'static Strings {
    strings(lang())
}

pub struct Strings {
    // Menus and windows
    pub menu_show: &'static str,
    pub menu_review: &'static str,
    pub menu_settings: &'static str,
    pub menu_quit: &'static str,
    pub menu_quit_app: &'static str,
    pub title_review: &'static str,
    pub title_settings: &'static str,

    // Notifications
    pub pause_over: (&'static str, &'static str),
    pub round_done: (&'static str, &'static str),
    pub break_over: (&'static str, &'static str),

    // Notes and reasons along the decision chain
    pub idle_but_media: &'static str,
    pub text_model_err: fn(&str) -> String,
    pub text_model_unsure: fn(f64, &str) -> String,
    pub shot_err: fn(&str) -> String,
    pub vision_model_err: fn(&str) -> String,
    pub vision_model_unsure: fn(&str) -> String,
    pub self_in_front: &'static str,
    pub away: fn(u64) -> String,
    pub undecided: &'static str,
    pub rule_round_allow: fn(&str) -> String,
    pub rule_block_app: fn(&str) -> String,
    pub rule_allow_app: fn(&str) -> String,
    pub rule_block_site: fn(&str) -> String,
    pub rule_allow_site: fn(&str) -> String,
    pub rule_block_keyword: fn(&str) -> String,
    pub rule_allow_keyword: fn(&str) -> String,
    pub decision_refused: &'static str,
    pub decision_no_choice: &'static str,
    pub decision_reason: fn(f64) -> String,

    // Errors
    pub err_page_without_url: &'static str,
    pub err_unknown_window: &'static str,
    pub err_unknown_slot: &'static str,
    pub err_model_not_set: &'static str,
    pub err_unknown_pane: &'static str,
    pub err_no_display: &'static str,
    pub err_blank_shot: &'static str,
    pub err_no_model_name: &'static str,
    pub err_no_key: &'static str,
    pub err_no_json: &'static str,
    pub err_bad_json: &'static str,
    pub err_timeout: &'static str,
    pub err_call: fn(&str) -> String,
    pub err_http: fn(u16, &str) -> String,
    pub err_bad_endpoint: &'static str,
    pub err_shot_size: fn(u32, u32, u32, u32) -> String,
    pub err_reply_not_json: &'static str,
    pub err_no_answer: &'static str,

    // What the models read
    pub prompt_system: &'static str,
    pub prompt_decision: &'static str,
    pub criteria_focused: &'static str,
    pub criteria_distracted: &'static str,
    pub state_window: &'static str,
    pub state_screenshot: &'static str,
    pub no_task: &'static str,
    pub round_allow_intro: &'static str,
    pub list_sep: &'static str,
    pub d_task: &'static str,
    pub d_app: &'static str,
    pub d_title: &'static str,
    pub d_url: &'static str,
    pub d_no_title: &'static str,
    pub d_no_url: &'static str,
    pub d_focused: &'static str,
    pub d_window_text: &'static str,
    pub d_document: &'static str,
    pub image_hint: &'static str,
    pub ping_task: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_system_tags() {
        let t = |s| Lang::from_tag(s);
        assert_eq!(t("zh-Hans-CN"), Some(Lang::ZhHans));
        assert_eq!(t("zh-CN"), Some(Lang::ZhHans));
        assert_eq!(t("zh-Hant-TW"), Some(Lang::ZhHant));
        assert_eq!(t("zh-HK"), Some(Lang::ZhHant));
        assert_eq!(t("zh-Hans-HK"), Some(Lang::ZhHans));
        assert_eq!(t("en-AU"), Some(Lang::EnAu));
        assert_eq!(t("en_GB"), Some(Lang::EnAu));
        assert_eq!(t("en"), Some(Lang::EnUs));
        assert_eq!(t("en-US"), Some(Lang::EnUs));
        assert_eq!(t("ja-JP"), Some(Lang::Ja));
        assert_eq!(t("fr-FR"), None);
    }

    #[test]
    fn unset_language_is_simplified_chinese() {
        assert_eq!(lang(), Lang::ZhHans);
    }
}
