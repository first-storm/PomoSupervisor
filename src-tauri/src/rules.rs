use crate::sampler::Sample;
use crate::settings::Rules;

/// A rule hit: (focused?, reason).
pub type RuleHit = (bool, String);

fn eq_ci(a: &str, b: &str) -> bool {
    a.trim().to_lowercase() == b.trim().to_lowercase()
}

/// `pattern` like `bilibili.com` or `*.bilibili.com` matches the host and its subdomains.
pub fn domain_matches(host: &str, pattern: &str) -> bool {
    let host = host.trim().trim_end_matches('.').to_lowercase();
    let p = pattern.trim().trim_start_matches("*.").trim_start_matches('.').to_lowercase();
    !p.is_empty() && (host == p || host.ends_with(&format!(".{p}")))
}

pub fn host_of(url: &str) -> Option<String> {
    url::Url::parse(url).ok()?.host_str().map(|h| h.to_lowercase())
}

pub fn evaluate(rules: &Rules, s: &Sample) -> Option<RuleHit> {
    let app_hit = |list: &[String]| {
        list.iter().find(|a| eq_ci(a, &s.app) || s.bundle_name.as_deref().is_some_and(|b| eq_ci(a, b))).cloned()
    };
    if let Some(a) = app_hit(&rules.block_apps) {
        return Some((false, format!("黑名单应用：{a}")));
    }
    if let Some(a) = app_hit(&rules.allow_apps) {
        return Some((true, format!("白名单应用：{a}")));
    }
    if let Some(host) = s.url.as_deref().and_then(host_of) {
        if let Some(d) = rules.block_domains.iter().find(|d| domain_matches(&host, d)) {
            return Some((false, format!("黑名单网站：{d}")));
        }
        if let Some(d) = rules.allow_domains.iter().find(|d| domain_matches(&host, d)) {
            return Some((true, format!("白名单网站：{d}")));
        }
    }
    let title = s.title.to_lowercase();
    let kw_hit = |list: &[String]| {
        list.iter().find(|k| !k.trim().is_empty() && title.contains(&k.trim().to_lowercase())).cloned()
    };
    if let Some(k) = kw_hit(&rules.block_keywords) {
        return Some((false, format!("标题含黑名单关键词：{k}")));
    }
    if let Some(k) = kw_hit(&rules.allow_keywords) {
        return Some((true, format!("标题含白名单关键词：{k}")));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(app: &str, title: &str, url: Option<&str>) -> Sample {
        Sample { app: app.into(), title: title.into(), url: url.map(Into::into), ..Default::default() }
    }

    #[test]
    fn domains_and_subdomains() {
        assert!(domain_matches("www.bilibili.com", "bilibili.com"));
        assert!(domain_matches("bilibili.com", "*.bilibili.com"));
        assert!(!domain_matches("notbilibili.com", "bilibili.com"));
    }

    #[test]
    fn order_of_rules() {
        let r = Rules::default();
        // An editor is allowed even when its title has a blocked keyword.
        assert_eq!(evaluate(&r, &sample("Code", "bilibili_scraper.rs", None)).unwrap().0, true);
        assert_eq!(evaluate(&r, &sample("Google Chrome", "x", Some("https://www.youtube.com/watch?v=1"))).unwrap().0, false);
        assert_eq!(evaluate(&r, &sample("Safari", "x", Some("https://docs.rs/tokio"))).unwrap().0, true);
        // Firefox gives no URL: the title keyword still catches it.
        assert_eq!(evaluate(&r, &sample("Firefox", "【游戏】哔哩哔哩 — Mozilla Firefox", None)).unwrap().0, false);
        assert!(evaluate(&r, &sample("Firefox", "Some random article", None)).is_none());
    }

    #[test]
    fn bundle_name_matches() {
        let r = Rules::default();
        let mut s = sample("Electron", "main.ts", None);
        s.bundle_name = Some("Visual Studio Code".into());
        assert_eq!(evaluate(&r, &s).unwrap().0, true);
    }
}
