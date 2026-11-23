use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind {
    OpenAI,
    Anthropic,
    Ollama,
    /// A decision model speaking TypeSafe's System One shape (TypeSafe, OpenRouter, AIMLAPI…).
    #[serde(rename = "decision")]
    Decision,
    /// OpenAI's Decisions API.
    #[serde(rename = "openai_decisions")]
    OpenAIDecisions,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProviderConfig {
    pub kind: ProviderKind,
    /// Empty means the provider's default endpoint. For decision models, the full endpoint URL.
    #[serde(default)]
    pub base_url: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Rules {
    pub block_apps: Vec<String>,
    pub allow_apps: Vec<String>,
    pub block_domains: Vec<String>,
    pub allow_domains: Vec<String>,
    pub block_keywords: Vec<String>,
    pub allow_keywords: Vec<String>,
}

fn v(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

impl Default for Rules {
    fn default() -> Self {
        Self {
            block_apps: v(&["Steam", "Netflix", "哔哩哔哩", "抖音", "TikTok"]),
            allow_apps: v(&[
                "Xcode", "Code", "Visual Studio Code", "Cursor", "Zed", "Terminal", "iTerm2",
                "Ghostty", "WezTerm", "kitty", "Alacritty", "IntelliJ IDEA", "PyCharm", "RustRover",
                "CLion", "WebStorm", "GoLand", "Obsidian", "Pages", "Keynote", "Numbers",
                "Microsoft Word", "Microsoft Excel", "Microsoft PowerPoint", "Figma",
            ]),
            block_domains: v(&[
                "bilibili.com", "youtube.com", "douyin.com", "tiktok.com", "weibo.com", "x.com",
                "twitter.com", "reddit.com", "xiaohongshu.com", "taobao.com", "jd.com",
                "netflix.com", "twitch.tv", "instagram.com", "facebook.com", "iqiyi.com",
                "youku.com", "huya.com", "douyu.com",
            ]),
            allow_domains: v(&[
                "github.com", "gitlab.com", "stackoverflow.com", "docs.rs", "crates.io",
                "developer.mozilla.org", "developer.apple.com", "tauri.app", "rust-lang.org",
                "arxiv.org",
            ]),
            block_keywords: v(&[
                "bilibili", "哔哩哔哩", "YouTube", "抖音", "微博", "小红书", "Netflix", "Twitch",
            ]),
            allow_keywords: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub focus_min: u32,
    pub short_break_min: u32,
    pub long_break_min: u32,
    pub long_break_every: u32,
    pub sample_secs: u32,
    pub idle_secs: u64,
    pub confidence: f64,
    pub phash_threshold: u32,
    pub streak: u32,
    pub sound: bool,
    /// When the person comes back from being away, resume on our own instead of waiting for a click.
    pub auto_resume: bool,
    pub text_enabled: bool,
    pub vision_enabled: bool,
    /// Read the frontmost window's text through the Accessibility API for the models.
    pub ax_enabled: bool,
    pub rules: Rules,
    pub text_model: Option<ProviderConfig>,
    pub vision_model: Option<ProviderConfig>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            focus_min: 25,
            short_break_min: 5,
            long_break_min: 15,
            long_break_every: 4,
            sample_secs: 4,
            idle_secs: 120,
            confidence: 0.75,
            phash_threshold: 6,
            streak: 3,
            sound: true,
            auto_resume: true,
            text_enabled: true,
            vision_enabled: false,
            ax_enabled: true,
            rules: Rules::default(),
            text_model: None,
            vision_model: None,
        }
    }
}

impl Settings {
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str::<Settings>(&s).ok())
            .unwrap_or_default()
            .sanitized()
    }

    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    /// Clamp values the UI could send out of range.
    pub fn sanitized(mut self) -> Self {
        self.focus_min = self.focus_min.clamp(1, 180);
        self.short_break_min = self.short_break_min.clamp(1, 60);
        self.long_break_min = self.long_break_min.clamp(1, 120);
        self.long_break_every = self.long_break_every.clamp(1, 12);
        self.sample_secs = self.sample_secs.clamp(2, 60);
        self.idle_secs = self.idle_secs.clamp(10, 3600);
        self.confidence = self.confidence.clamp(0.0, 1.0);
        self.phash_threshold = self.phash_threshold.clamp(0, 64);
        self.streak = self.streak.clamp(1, 20);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_kinds_round_trip() {
        for (kind, name) in [
            (ProviderKind::OpenAI, "openai"),
            (ProviderKind::Ollama, "ollama"),
            (ProviderKind::Decision, "decision"),
            (ProviderKind::OpenAIDecisions, "openai_decisions"),
        ] {
            assert_eq!(serde_json::to_string(&kind).unwrap(), format!("\"{name}\""));
            assert_eq!(serde_json::from_str::<ProviderKind>(&format!("\"{name}\"")).unwrap(), kind);
        }
        let old = r#"{"text_model":{"kind":"anthropic","model":"claude-haiku-5-5"}}"#;
        let s: Settings = serde_json::from_str(old).unwrap();
        assert_eq!(s.text_model.unwrap().kind, ProviderKind::Anthropic);
    }
}
