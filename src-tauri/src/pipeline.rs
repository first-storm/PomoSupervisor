//! The decision chain: stages run in order and the first conclusion wins.
use futures::future::BoxFuture;

/// The stage that settled a sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decider {
    /// This round's allow list.
    Round,
    Rules,
    Text,
    Vision,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Verdict {
    pub focused: bool,
    pub by: Decider,
    pub reason: String,
    pub confidence: f64,
    /// Reused a cached / de-duplicated model answer.
    pub cached: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StageResult {
    /// The person has left: pause, don't count.
    Away,
    Decided(Verdict),
    /// No conclusion here; an optional note says why (e.g. a model error).
    Pass(Option<String>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    Away,
    Decided(Verdict),
    Undecided,
}

pub type Stage = Box<dyn FnOnce() -> BoxFuture<'static, StageResult> + Send>;

/// Run stages lazily in order; later stages are never started once one concludes.
pub async fn run_chain(stages: Vec<Stage>) -> (Outcome, Vec<String>) {
    let mut notes = Vec::new();
    for stage in stages {
        match stage().await {
            StageResult::Away => return (Outcome::Away, notes),
            StageResult::Decided(v) => return (Outcome::Decided(v), notes),
            StageResult::Pass(note) => notes.extend(note),
        }
    }
    (Outcome::Undecided, notes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn stage(r: StageResult, calls: Arc<AtomicU32>) -> Stage {
        Box::new(move || {
            Box::pin(async move {
                calls.fetch_add(1, Ordering::SeqCst);
                r
            })
        })
    }

    fn decided(by: Decider) -> StageResult {
        StageResult::Decided(Verdict { focused: false, by, reason: String::new(), confidence: 1.0, cached: false })
    }

    #[tokio::test]
    async fn stops_at_first_conclusion() {
        let calls = Arc::new(AtomicU32::new(0));
        let (out, notes) = run_chain(vec![
            stage(StageResult::Pass(None), calls.clone()),
            stage(StageResult::Pass(Some("文本模型没把握".into())), calls.clone()),
            stage(decided(Decider::Vision), calls.clone()),
            stage(decided(Decider::Rules), calls.clone()),
        ])
        .await;
        assert!(matches!(out, Outcome::Decided(Verdict { by: Decider::Vision, .. })));
        assert_eq!(calls.load(Ordering::SeqCst), 3);
        assert_eq!(notes, vec!["文本模型没把握".to_string()]);
    }

    #[tokio::test]
    async fn away_short_circuits() {
        let calls = Arc::new(AtomicU32::new(0));
        let (out, _) = run_chain(vec![stage(StageResult::Away, calls.clone()), stage(decided(Decider::Rules), calls.clone())]).await;
        assert_eq!(out, Outcome::Away);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn all_pass_is_undecided() {
        let calls = Arc::new(AtomicU32::new(0));
        let (out, _) = run_chain(vec![stage(StageResult::Pass(None), calls)]).await;
        assert_eq!(out, Outcome::Undecided);
    }
}
