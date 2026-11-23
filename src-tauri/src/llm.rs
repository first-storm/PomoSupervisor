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

pub fn build(cfg: &ProviderConfig, key: Option<String>) -> anyhow::Result<ChatModel> {
    let model = cfg.model.trim().to_string();
    if model.is_empty() {
        return Err(anyhow!("未填写模型名"));
    }
    let base = cfg.base_url.trim().to_string();
    Ok(match cfg.kind {
        ProviderKind::OpenAI => {
            let mut c = rig_core::providers::openai::OpenAIConfig::new(key.context("缺少 API Key")?);
            if !base.is_empty() {
                c = c.with_base_url(base);
            }
            c.client().chat(model).erase()
        }
        ProviderKind::Anthropic => {
            let mut c = rig_core::providers::anthropic::AnthropicConfig::new(key.context("缺少 API Key")?);
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
    })
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
    let start = text.find('{').context("回复里没有 JSON")?;
    let end = text.rfind('}').context("回复里没有 JSON")?;
    let mut j: Judgement = serde_json::from_str(&text[start..=end]).context("JSON 解析失败")?;
    j.verdict = j.verdict.trim().to_lowercase();
    if !matches!(j.verdict.as_str(), "focused" | "distracted") {
        j.verdict = "unsure".into();
    }
    j.confidence = j.confidence.clamp(0.0, 1.0);
    Ok(j)
}

const SYSTEM: &str = "你是番茄钟专注监督员。根据用户本轮声明的任务和当前前台窗口的信息，判断用户此刻是否在做与任务相关的事。\
只输出一个 JSON 对象，不要任何其他文字：{\"verdict\":\"focused|distracted|unsure\",\"confidence\":0到1的小数,\"reason\":\"不超过20字的中文理由\"}。\
查资料、看文档、写代码、与任务相关的沟通算 focused；娱乐视频、社交媒体刷帖、购物、游戏、与任务无关的内容算 distracted；信息不足以判断时输出 unsure 并给出较低的 confidence。";

fn describe(task: &str, s: &Sample) -> String {
    let task = if task.trim().is_empty() { "（用户未填写任务，按一般工作/学习判断）" } else { task.trim() };
    format!(
        "本轮任务：{task}\n前台应用：{}\n窗口标题：{}\nURL：{}",
        s.bundle_name.as_deref().unwrap_or(&s.app),
        if s.title.is_empty() { "（无）" } else { &s.title },
        s.url.as_deref().unwrap_or("（无法获取）")
    )
}

async fn ask(model: &ChatModel, prompt: Message) -> anyhow::Result<Judgement> {
    let req = CompletionRequest::new(prompt).preamble(SYSTEM).max_tokens(400u64);
    let resp = tokio::time::timeout(Duration::from_secs(20), model.call(req))
        .await
        .map_err(|_| anyhow!("模型调用超时"))?
        .map_err(|e| anyhow!("模型调用失败：{e}"))?;
    parse_judgement(&resp.text())
}

pub async fn judge_text(model: &ChatModel, task: &str, s: &Sample) -> anyhow::Result<Judgement> {
    ask(model, Message::user(describe(task, s))).await
}

pub async fn judge_image(model: &ChatModel, task: &str, s: &Sample, jpeg_b64: String) -> anyhow::Result<Judgement> {
    let text = format!("{}\n附图是当前屏幕截图，请结合截图内容判断。", describe(task, s));
    let msg = Message::from(vec![
        UserContent::text(text),
        UserContent::image_base64(jpeg_b64, Some(ImageMediaType::JPEG), None),
    ]);
    ask(model, msg).await
}

/// A tiny request to check the provider settings work.
pub async fn ping(model: &ChatModel) -> anyhow::Result<String> {
    let s = Sample { app: "Code".into(), title: "main.rs — PomoSupervisor".into(), ..Default::default() };
    let j = judge_text(model, "写 Rust 代码", &s).await?;
    Ok(format!("{}（{:.2}）{}", j.verdict, j.confidence, j.reason))
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
    fn no_json_is_error() {
        assert!(parse_judgement("I think focused").is_err());
    }
}
