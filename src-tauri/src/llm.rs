use crate::ax::AxInfo;
use crate::decision::{self, DecisionModel, Dialect};
use crate::sampler::Sample;
use crate::settings::{ProviderConfig, ProviderKind};
use anyhow::{Context, anyhow};
use rig_core::DynModel;
use rig_core::completion::CompletionRequest;
use rig_core::message::{ImageMediaType, Message, UserContent};
use rig_core::operation::Completion;
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub type ChatModel = DynModel<Completion>;

/// What judges a sample: a chat model (rig) that writes a JSON verdict, or a decision model that scores the options.
pub enum Judge {
    Chat(ChatModel),
    Decision(DecisionModel),
}

/// A screenshot as the judges take it.
pub struct Picture {
    pub jpeg_b64: String,
    pub width: u32,
    pub height: u32,
}

const KEYCHAIN_SERVICE: &str = "PomoSupervisor";

pub fn get_key(slot: &str) -> Option<String> {
    keyring::Entry::new(KEYCHAIN_SERVICE, slot).ok()?.get_password().ok().filter(|k| !k.is_empty())
}

pub fn set_key(slot: &str, key: &str) -> anyhow::Result<()> {
    let entry = keyring::Entry::new(KEYCHAIN_SERVICE, slot)?;
    if key.is_empty() {
        let _ = entry.delete_credential();
    } else {
        entry.set_password(key)?;
    }
    Ok(())
}

pub fn build(cfg: &ProviderConfig, key: Option<String>) -> anyhow::Result<Judge> {
    let tr = crate::i18n::tr();
    let model = cfg.model.trim().to_string();
    if model.is_empty() {
        return Err(anyhow!(tr.err_no_model_name));
    }
    let base = cfg.base_url.trim().to_string();
    let decision = |dialect| DecisionModel::new(dialect, &base, key.clone().context(tr.err_no_key)?, model.clone()).map(Judge::Decision);
    Ok(Judge::Chat(match cfg.kind {
        ProviderKind::Decision => return decision(Dialect::SystemOne),
        ProviderKind::OpenAIDecisions => return decision(Dialect::OpenAI),
        ProviderKind::OpenAI => {
            let mut c = rig_core::providers::openai::OpenAIConfig::new(key.context(tr.err_no_key)?);
            if !base.is_empty() {
                c = c.with_base_url(base);
            }
            c.client().chat(model).erase()
        }
        ProviderKind::Anthropic => {
            let mut c = rig_core::providers::anthropic::AnthropicConfig::new(key.context(tr.err_no_key)?);
            if !base.is_empty() {
                c = c.with_base_url(base);
            }
            c.client().completion(model).erase()
        }
        ProviderKind::Ollama => {
            let mut c = rig_core::providers::ollama::OllamaConfig::new();
            if !base.is_empty() {
                // rig appends /v1 itself; accept a pasted ".../v1" too.
                c = c.with_base_url(base.trim_end_matches('/').trim_end_matches("/v1"));
            }
            c.client().completion(model).erase()
        }
    }))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Judgement {
    /// "focused" | "distracted" | "unsure"
    pub verdict: String,
    #[serde(default)]
    pub confidence: f64,
    #[serde(default)]
    pub reason: String,
}

impl Judgement {
    pub fn focused(&self) -> Option<bool> {
        match self.verdict.as_str() {
            "focused" => Some(true),
            "distracted" => Some(false),
            _ => None,
        }
    }
}

/// Pull the JSON object out of a model reply (tolerates code fences and prose).
pub fn parse_judgement(text: &str) -> anyhow::Result<Judgement> {
    let tr = crate::i18n::tr();
    let start = text.find('{').context(tr.err_no_json)?;
    let end = text.rfind('}').context(tr.err_no_json)?;
    let mut j: Judgement = serde_json::from_str(&text[start..=end]).context(tr.err_bad_json)?;
    j.verdict = j.verdict.trim().to_lowercase();
    if !matches!(j.verdict.as_str(), "focused" | "distracted") {
        j.verdict = "unsure".into();
    }
    j.confidence = j.confidence.clamp(0.0, 1.0);
    Ok(j)
}

/// The task as the models get it: what the person wrote, plus what they have said belongs
/// to it this round, so one correction also informs the judgement of similar windows.
pub fn task_context(task: &str, allow: &[String]) -> String {
    if allow.is_empty() {
        return task.trim().to_string();
    }
    let tr = crate::i18n::tr();
    let task = if task.trim().is_empty() { tr.no_task } else { task.trim() };
    format!("{task}\n{}{}", tr.round_allow_intro, allow.join(tr.list_sep))
}

fn describe(task: &str, s: &Sample, ax: Option<&AxInfo>) -> String {
    let tr = crate::i18n::tr();
    let task = if task.trim().is_empty() { tr.no_task } else { task.trim() };
    let url = s.url.as_deref().or(ax.and_then(|a| a.url.as_deref()));
    let mut out = format!(
        "{}{task}\n{}{}\n{}{}\n{}{}",
        tr.d_task,
        tr.d_app,
        s.bundle_name.as_deref().unwrap_or(&s.app),
        tr.d_title,
        if s.title.is_empty() { tr.d_no_title } else { &s.title },
        tr.d_url,
        url.unwrap_or(tr.d_no_url)
    );
    if let Some(f) = ax.and_then(|a| a.focused.as_deref()) {
        out.push_str(&format!("\n{}{f}", tr.d_focused));
    }
    if let Some(t) = ax.and_then(|a| a.text.as_deref()) {
        out.push_str(&format!("\n{}\n{t}", tr.d_window_text));
    }
    out
}

async fn ask(model: &ChatModel, prompt: Message) -> anyhow::Result<Judgement> {
    let tr = crate::i18n::tr();
    let req = CompletionRequest::new(prompt).preamble(tr.prompt_system).max_tokens(400u64);
    let resp = tokio::time::timeout(Duration::from_secs(20), model.call(req))
        .await
        .map_err(|_| anyhow!(tr.err_timeout))?
        .map_err(|e| anyhow!((tr.err_call)(&e.to_string())))?;
    parse_judgement(&resp.text())
}

/// `min_conf` only matters for decision models: below it their pick becomes "unsure".
pub async fn judge_text(model: &Judge, task: &str, s: &Sample, ax: Option<&AxInfo>, min_conf: f64) -> anyhow::Result<Judgement> {
    let text = describe(task, s, ax);
    match model {
        Judge::Chat(m) => ask(m, Message::user(text)).await,
        Judge::Decision(d) => d.judge(crate::i18n::tr().prompt_decision, &text, None, min_conf).await,
    }
}

pub async fn judge_image(model: &Judge, task: &str, s: &Sample, ax: Option<&AxInfo>, pic: Picture, min_conf: f64) -> anyhow::Result<Judgement> {
    match model {
        Judge::Chat(m) => {
            let text = format!("{}\n{}", describe(task, s, ax), crate::i18n::tr().image_hint);
            let msg = Message::from(vec![
                UserContent::text(text),
                UserContent::image_base64(pic.jpeg_b64, Some(ImageMediaType::JPEG), None),
            ]);
            ask(m, msg).await
        }
        Judge::Decision(d) => {
            let img = decision::Image { jpeg_b64: &pic.jpeg_b64, width: pic.width, height: pic.height };
            d.judge(crate::i18n::tr().prompt_decision, &describe(task, s, ax), Some(img), min_conf).await
        }
    }
}

fn ping_sample() -> Sample {
    Sample { app: "Code".into(), title: "main.rs — PomoSupervisor".into(), ..Default::default() }
}

fn show(j: Judgement) -> String {
    format!("{}（{:.2}）{}", j.verdict, j.confidence, j.reason)
}

/// A tiny request to check the provider settings work.
pub async fn ping(model: &Judge) -> anyhow::Result<String> {
    Ok(show(judge_text(model, crate::i18n::tr().ping_task, &ping_sample(), None, 0.0).await?))
}

/// Like `ping`, with a small plain image, so a model or endpoint that can't take images fails here.
pub async fn ping_image(model: &Judge) -> anyhow::Result<String> {
    let img = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(320, 320, image::Rgb([40, 44, 52])));
    let pic = Picture { jpeg_b64: crate::vision::encode_jpeg(&img)?, width: 320, height: 320 };
    Ok(show(judge_image(model, crate::i18n::tr().ping_task, &ping_sample(), None, pic, 0.0).await?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fenced_json() {
        let j = parse_judgement("```json\n{\"verdict\":\"Distracted\",\"confidence\":0.9,\"reason\":\"刷视频\"}\n```").unwrap();
        assert_eq!(j.focused(), Some(false));
        assert_eq!(j.confidence, 0.9);
    }

    #[test]
    fn unknown_verdict_is_unsure() {
        let j = parse_judgement("{\"verdict\":\"maybe\",\"confidence\":3}").unwrap();
        assert_eq!(j.focused(), None);
        assert_eq!(j.confidence, 1.0);
    }

    #[test]
    fn describe_adds_ax_and_falls_back_to_its_url() {
        let s = Sample { app: "Firefox".into(), title: "文档".into(), ..Default::default() };
        let plain = describe("写论文", &s, None);
        assert!(plain.contains("URL：（无法获取）") && !plain.contains("窗口内容"));
        let ax = AxInfo { focused: Some("AXTextField 搜索".into()), text: Some("第一段".into()), url: Some("https://example.com/a".into()) };
        let d = describe("写论文", &s, Some(&ax));
        assert!(d.contains("URL：https://example.com/a"));
        assert!(d.contains("焦点控件：AXTextField 搜索"));
        assert!(d.ends_with("窗口内容（辅助功能读取，可能不完整）：\n第一段"));
        let with_url = Sample { url: Some("https://own.example".into()), ..s };
        assert!(describe("写论文", &with_url, Some(&ax)).contains("URL：https://own.example"));
    }

    #[test]
    fn task_context_carries_the_round_allow_list() {
        assert_eq!(task_context(" 写论文 ", &[]), "写论文");
        assert_eq!(task_context("", &[]), "");
        let c = task_context("", &["figma.com".into(), "微信".into()]);
        assert!(c.starts_with(crate::i18n::tr().no_task) && c.ends_with("figma.com、微信"));
    }

    #[test]
    fn no_json_is_error() {
        assert!(parse_judgement("I think focused").is_err());
    }
}
