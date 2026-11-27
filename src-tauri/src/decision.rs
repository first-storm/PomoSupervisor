//! Decision models: endpoints that answer a fixed question with option probabilities instead of text.
//! Two request shapes exist: TypeSafe's System One (also OpenRouter, AIMLAPI) and OpenAI's Decisions API.
use crate::llm::Judgement;
use anyhow::{Context, anyhow};
use serde_json::{Value, json};
use std::time::Duration;

pub const SYSTEMONE_DEFAULT: &str = "https://api.typesafe.ai/v1/systemone";
pub const OPENAI_DEFAULT: &str = "https://api.openai.com/v1/decisions";
/// Per-side image bounds the decision APIs accept; they don't resize for you.
pub const IMG_MIN: u32 = 280;
pub const IMG_MAX: u32 = 1568;

const QUESTION: &str = "verdict";
/// Asset key; it is matched against strings in `state`, so it must not look like a window title.
const ASSET: &str = "pomo-shot-7f3c2a9e-4b1d-4e8a-9c55-1d2e3f4a5b6c";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialect {
    SystemOne,
    OpenAI,
}

/// How a System One gateway takes an image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImageMode {
    /// TypeSafe, AIMLAPI: an `assets` map referenced by key from `state`.
    Assets,
    /// OpenRouter: an `image_url` part in a top-level `state` array (assets work for Jev only there).
    ImageUrl,
}

pub struct Image<'a> {
    pub jpeg_b64: &'a str,
    pub width: u32,
    pub height: u32,
}

pub struct DecisionModel {
    http: reqwest::Client,
    endpoint: String,
    key: String,
    model: String,
    dialect: Dialect,
}

impl DecisionModel {
    pub fn new(dialect: Dialect, base_url: &str, key: String, model: String) -> anyhow::Result<Self> {
        let endpoint = match base_url.trim() {
            "" if dialect == Dialect::SystemOne => SYSTEMONE_DEFAULT.to_string(),
            "" => OPENAI_DEFAULT.to_string(),
            u => u.to_string(),
        };
        url::Url::parse(&endpoint).context(crate::i18n::tr().err_bad_endpoint)?;
        let http = reqwest::Client::builder().timeout(Duration::from_secs(20)).build()?;
        Ok(Self { http, endpoint, key, model, dialect })
    }

    fn image_mode(&self) -> ImageMode {
        let host = url::Url::parse(&self.endpoint).ok().and_then(|u| u.host_str().map(str::to_lowercase));
        if host.is_some_and(|h| h == "openrouter.ai" || h.ends_with(".openrouter.ai")) { ImageMode::ImageUrl } else { ImageMode::Assets }
    }

    /// The request body, and whether it carries an image.
    fn build_body(&self, instructions: &str, text: &str, jpeg_b64: Option<&str>) -> (Value, bool) {
        let tr = crate::i18n::tr();
        let body = match self.dialect {
            Dialect::SystemOne => {
                let question = json!({
                    "type": "choice",
                    "instructions": instructions,
                    "criteria": { "focused": tr.criteria_focused, "distracted": tr.criteria_distracted },
                });
                let mut body = json!({ "model": self.model, "questions": { QUESTION: question } });
                match (jpeg_b64, self.image_mode()) {
                    (None, _) => body["state"] = json!(text),
                    (Some(b64), ImageMode::Assets) => {
                        body["state"] = json!({ tr.state_window: text, tr.state_screenshot: ASSET });
                        body["assets"] = json!({ ASSET: { "type": "image", "format": "jpg", "data_base64": b64 } });
                    }
                    (Some(b64), ImageMode::ImageUrl) => {
                        body["state"] = json!([text, { "type": "image_url", "image_url": { "url": format!("data:image/jpeg;base64,{b64}") } }]);
                    }
                }
                body
            }
            Dialect::OpenAI => {
                let input = match jpeg_b64 {
                    None => json!(text),
                    Some(b64) => json!([{ "role": "user", "content": [
                        { "type": "input_text", "text": text },
                        { "type": "input_image", "image_url": format!("data:image/jpeg;base64,{b64}") },
                    ]}]),
                };
                json!({
                    "model": self.model,
                    "input": input,
                    "questions": [{
                        "type": "choice",
                        "name": QUESTION,
                        "instructions": instructions,
                        "choices": [
                            { "value": "focused", "description": tr.criteria_focused },
                            { "value": "distracted", "description": tr.criteria_distracted },
                        ],
                    }],
                })
            }
        };
        (body, jpeg_b64.is_some())
    }

    pub async fn judge(&self, instructions: &str, text: &str, image: Option<Image<'_>>, min_conf: f64) -> anyhow::Result<Judgement> {
        let tr = crate::i18n::tr();
        if let Some(img) = &image {
            let ok = |n: u32| (IMG_MIN..=IMG_MAX).contains(&n);
            if !ok(img.width) || !ok(img.height) {
                return Err(anyhow!((tr.err_shot_size)(img.width, img.height, IMG_MIN, IMG_MAX)));
            }
        }
        let (body, has_image) = self.build_body(instructions, text, image.as_ref().map(|i| i.jpeg_b64));
        let mut req = self.http.post(&self.endpoint).bearer_auth(&self.key).json(&body);
        if has_image && self.dialect == Dialect::SystemOne && self.image_mode() == ImageMode::Assets {
            req = req.header("TypeSafe-Danger-Zone-Images", "true");
        }
        let resp = req.send().await.map_err(|e| if e.is_timeout() { anyhow!(tr.err_timeout) } else { anyhow!((tr.err_call)(&e.to_string())) })?;
        let status = resp.status();
        let text = resp.text().await.map_err(|e| anyhow!((tr.err_call)(&e.to_string())))?;
        if !status.is_success() {
            let snippet: String = text.chars().take(200).collect();
            return Err(anyhow!((tr.err_http)(status.as_u16(), &snippet)));
        }
        let v: Value = serde_json::from_str(&text).context(tr.err_reply_not_json)?;
        parse_answer(self.dialect, &v, min_conf)
    }
}

fn verdict_of(choice: &Value) -> Option<&'static str> {
    match choice.as_str()?.trim().to_lowercase().as_str() {
        "focused" => Some("focused"),
        "distracted" => Some("distracted"),
        _ => None,
    }
}

/// Probability the answer gives `name` (System One: a map; OpenAI: a list of {value, probability}).
fn prob_of(answer: &Value, name: &str) -> Option<f64> {
    match &answer["probabilities"] {
        Value::Object(m) => m.get(name).and_then(Value::as_f64),
        Value::Array(a) => a.iter().find(|p| p["value"].as_str() == Some(name)).and_then(|p| p["probability"].as_f64()),
        _ => None,
    }
}

/// Turn a decision response into a Judgement. Below `min_conf` it is "unsure": with two options the model
/// always picks one, and a near coin-flip must not count as a distraction.
pub fn parse_answer(dialect: Dialect, v: &Value, min_conf: f64) -> anyhow::Result<Judgement> {
    let tr = crate::i18n::tr();
    let answer = match dialect {
        Dialect::SystemOne => v["answers"].get(QUESTION),
        Dialect::OpenAI => v["answers"]
            .as_array()
            .and_then(|a| a.iter().find(|x| x["name"].as_str() == Some(QUESTION)).or_else(|| a.first())),
    }
    .context(tr.err_no_answer)?;
    let unsure = |reason: &str| Judgement { verdict: "unsure".into(), confidence: 0.0, reason: reason.into() };
    if answer["type"].as_str() == Some("refusal") {
        return Ok(unsure(tr.decision_refused));
    }
    let Some(verdict) = verdict_of(&answer["choice"]) else { return Ok(unsure(tr.decision_no_choice)) };
    let confidence = answer["confidence"].as_f64().or_else(|| prob_of(answer, verdict)).unwrap_or(0.0).clamp(0.0, 1.0);
    let p_distracted = prob_of(answer, "distracted").unwrap_or(if verdict == "distracted" { confidence } else { 1.0 - confidence });
    let reason = (tr.decision_reason)(p_distracted);
    let verdict = if confidence < min_conf { "unsure" } else { verdict };
    Ok(Judgement { verdict: verdict.into(), confidence, reason })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model(dialect: Dialect, url: &str) -> DecisionModel {
        DecisionModel::new(dialect, url, "k".into(), "m".into()).unwrap()
    }

    #[test]
    fn default_endpoints_and_bad_url() {
        assert_eq!(model(Dialect::SystemOne, "").endpoint, SYSTEMONE_DEFAULT);
        assert_eq!(model(Dialect::OpenAI, " ").endpoint, OPENAI_DEFAULT);
        assert!(DecisionModel::new(Dialect::SystemOne, "not a url", "k".into(), "m".into()).is_err());
    }

    #[test]
    fn systemone_text_body() {
        let (b, img) = model(Dialect::SystemOne, "").build_body("判断", "窗口", None);
        assert!(!img);
        assert_eq!(b["state"], "窗口");
        assert_eq!(b["questions"]["verdict"]["type"], "choice");
        assert!(b["questions"]["verdict"]["criteria"]["distracted"].is_string());
        assert!(b.get("assets").is_none());
    }

    #[test]
    fn systemone_assets_image_body() {
        let m = model(Dialect::SystemOne, "https://api.aimlapi.com/v1/decisions");
        assert_eq!(m.image_mode(), ImageMode::Assets);
        let (b, img) = m.build_body("判断", "窗口", Some("QUJD"));
        assert!(img);
        assert_eq!(b["state"]["当前屏幕截图"], ASSET);
        assert_eq!(b["assets"][ASSET]["data_base64"], "QUJD");
        assert_eq!(b["assets"][ASSET]["format"], "jpg");
    }

    #[test]
    fn openrouter_image_url_body() {
        let m = model(Dialect::SystemOne, "https://openrouter.ai/api/alpha/decisions");
        assert_eq!(m.image_mode(), ImageMode::ImageUrl);
        let (b, _) = m.build_body("判断", "窗口", Some("QUJD"));
        assert_eq!(b["state"][0], "窗口");
        assert_eq!(b["state"][1]["image_url"]["url"], "data:image/jpeg;base64,QUJD");
        assert!(b.get("assets").is_none());
    }

    #[test]
    fn openai_bodies() {
        let m = model(Dialect::OpenAI, "");
        let (t, _) = m.build_body("判断", "窗口", None);
        assert_eq!(t["input"], "窗口");
        assert_eq!(t["questions"][0]["name"], "verdict");
        assert_eq!(t["questions"][0]["choices"][1]["value"], "distracted");
        let (i, _) = m.build_body("判断", "窗口", Some("QUJD"));
        assert_eq!(i["input"][0]["content"][1]["type"], "input_image");
        assert_eq!(i["input"][0]["content"][1]["image_url"], "data:image/jpeg;base64,QUJD");
    }

    #[test]
    fn parses_systemone_choice() {
        let v = json!({"answers":{"verdict":{"type":"choice","choice":"distracted","confidence":0.91,"probabilities":{"focused":0.09,"distracted":0.91}}}});
        let j = parse_answer(Dialect::SystemOne, &v, 0.75).unwrap();
        assert_eq!(j.focused(), Some(false));
        assert_eq!(j.confidence, 0.91);
        assert_eq!(j.reason, "决策模型：分心概率 0.91");
    }

    #[test]
    fn parses_openai_choice_without_confidence() {
        let v = json!({"answers":[{"type":"choice","name":"verdict","choice":"focused","probabilities":[{"value":"focused","probability":0.8},{"value":"distracted","probability":0.2}]}]});
        let j = parse_answer(Dialect::OpenAI, &v, 0.75).unwrap();
        assert_eq!(j.focused(), Some(true));
        assert_eq!(j.confidence, 0.8);
        assert_eq!(j.reason, "决策模型：分心概率 0.20");
    }

    #[test]
    fn low_confidence_and_refusal_are_unsure() {
        let v = json!({"answers":{"verdict":{"type":"choice","choice":"distracted","confidence":0.52}}});
        let j = parse_answer(Dialect::SystemOne, &v, 0.75).unwrap();
        assert_eq!(j.focused(), None);
        assert_eq!(j.confidence, 0.52);
        let r = json!({"answers":[{"type":"refusal","name":"verdict"}]});
        assert_eq!(parse_answer(Dialect::OpenAI, &r, 0.5).unwrap().focused(), None);
        assert!(parse_answer(Dialect::SystemOne, &json!({"error":"x"}), 0.5).is_err());
    }
}
