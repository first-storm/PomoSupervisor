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

/// A page as an allow entry holds it: the address without its fragment or trailing slash.
pub fn page_key(url: &str) -> Option<String> {
    let mut u = url::Url::parse(url).ok()?;
    u.host_str()?;
    u.set_fragment(None);
    Some(u.as_str().trim_end_matches('/').to_string())
}

/// Apps, sites and single pages (entries with a scheme) allowed for this round only.
/// They win over the block lists.
pub fn round_allow(extra: &[String], s: &Sample) -> Option<RuleHit> {
    let host = s.url.as_deref().and_then(host_of);
    let page = s.url.as_deref().and_then(page_key);
    extra
        .iter()
        .find(|e| {
            if e.contains("://") {
                return page.is_some() && page_key(e) == page;
            }
            eq_ci(e, &s.app)
                || s.bundle_name.as_deref().is_some_and(|b| eq_ci(e, b))
                || host.as_deref().is_some_and(|h| e.contains('.') && domain_matches(h, e))
        })
        .map(|e| (true, (crate::i18n::tr().rule_round_allow)(e)))
}

/// Browsers by app name, localized names included ("Safari浏览器").
pub fn is_browser(app: &str) -> bool {
    let a = app.trim().to_lowercase();
    ["safari", "chrome", "chromium", "firefox", "microsoft edge", "brave", "opera", "vivaldi", "browser", "浏览器", "瀏覽器", "ブラウザ"]
        .iter()
        .any(|b| a.contains(b))
        || ["arc", "orion", "zen", "dia", "edge", "helium", "librewolf", "waterfox", "floorp", "thorium"].contains(&a.as_str())
}

/// What to call the window a sample saw: its site, else its app.
pub fn target_of(app: &str, url: Option<&str>) -> String {
    match url.and_then(host_of) {
        Some(h) => h.strip_prefix("www.").unwrap_or(&h).to_string(),
        None => app.to_string(),
    }
}

/// The entry to whitelist for a sample: its site when there is a URL, else its app.
/// None for a browser without a readable URL: allowing the whole browser would switch
/// supervision off for everything in it.
pub fn allow_target(app: &str, url: Option<&str>) -> Option<String> {
    match url.and_then(host_of) {
        Some(h) => Some(h.strip_prefix("www.").unwrap_or(&h).to_string()),
        None if is_browser(app) => None,
        None => Some(app.to_string()),
    }
}

/// What "this is part of the task" allows during a round. A site on the block list is a
/// known time sink, so only the page in front is let through, not the whole site.
pub fn round_allow_target(rules: &Rules, app: &str, url: Option<&str>) -> Option<String> {
    match url.and_then(host_of) {
        Some(h) if rules.block_domains.iter().any(|d| domain_matches(&h, d)) => url.and_then(page_key),
        _ => allow_target(app, url),
    }
}

/// Move `target` from the block lists to the allow list (domain or app, by shape).
pub fn mark_as_work(rules: &mut Rules, target: &str) {
    let t = target.trim();
    if t.is_empty() {
        return;
    }
    if t.contains('.') && !t.contains(' ') {
        rules.block_domains.retain(|d| !domain_matches(t, d));
        if !rules.allow_domains.iter().any(|d| domain_matches(t, d)) {
            rules.allow_domains.push(t.to_string());
        }
    } else {
        rules.block_apps.retain(|a| !eq_ci(a, t));
        if !rules.allow_apps.iter().any(|a| eq_ci(a, t)) {
            rules.allow_apps.push(t.to_string());
        }
    }
}

pub fn evaluate(rules: &Rules, s: &Sample) -> Option<RuleHit> {
    let tr = crate::i18n::tr();
    let app_hit =
        |list: &[String]| list.iter().find(|a| eq_ci(a, &s.app) || s.bundle_name.as_deref().is_some_and(|b| eq_ci(a, b))).cloned();
    if let Some(a) = app_hit(&rules.block_apps) {
        return Some((false, (tr.rule_block_app)(&a)));
    }
    if let Some(a) = app_hit(&rules.allow_apps) {
        return Some((true, (tr.rule_allow_app)(&a)));
    }
    if let Some(host) = s.url.as_deref().and_then(host_of) {
        if let Some(d) = rules.block_domains.iter().find(|d| domain_matches(&host, d)) {
            return Some((false, (tr.rule_block_site)(d)));
        }
        if let Some(d) = rules.allow_domains.iter().find(|d| domain_matches(&host, d)) {
            return Some((true, (tr.rule_allow_site)(d)));
        }
    }
    let title = s.title.to_lowercase();
    let kw_hit = |list: &[String]| list.iter().find(|k| !k.trim().is_empty() && title.contains(&k.trim().to_lowercase())).cloned();
    if let Some(k) = kw_hit(&rules.block_keywords) {
        return Some((false, (tr.rule_block_keyword)(&k)));
    }
    if let Some(k) = kw_hit(&rules.allow_keywords) {
        return Some((true, (tr.rule_allow_keyword)(&k)));
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
        assert!(evaluate(&r, &sample("Code", "bilibili_scraper.rs", None)).unwrap().0);
        assert!(!evaluate(&r, &sample("Google Chrome", "x", Some("https://www.youtube.com/watch?v=1"))).unwrap().0);
        assert!(evaluate(&r, &sample("Safari", "x", Some("https://docs.rs/tokio"))).unwrap().0);
        // Firefox gives no URL: the title keyword still catches it.
        assert!(!evaluate(&r, &sample("Firefox", "【游戏】哔哩哔哩 — Mozilla Firefox", None)).unwrap().0);
        assert!(evaluate(&r, &sample("Firefox", "Some random article", None)).is_none());
    }

    #[test]
    fn round_allow_beats_block_list() {
        let s = sample("Safari", "x", Some("https://www.bilibili.com/video/1"));
        assert!(round_allow(&["bilibili.com".into()], &s).unwrap().0);
        assert!(round_allow(&["Terminal".into()], &s).is_none());
        assert!(round_allow(&["safari".into()], &s).is_some());
    }

    #[test]
    fn a_blocked_site_is_allowed_one_page_at_a_time() {
        let r = Rules::default();
        let url = "https://www.youtube.com/watch?v=abc#t=10";
        let entry = round_allow_target(&r, "Safari", Some(url)).unwrap();
        assert_eq!(entry, "https://www.youtube.com/watch?v=abc");
        assert!(round_allow(std::slice::from_ref(&entry), &sample("Safari", "x", Some(url))).unwrap().0);
        assert!(round_allow(std::slice::from_ref(&entry), &sample("Safari", "x", Some("https://www.youtube.com/watch?v=other"))).is_none());
        assert!(round_allow(&[entry], &sample("Safari", "x", None)).is_none());
        // A site nobody blocked is allowed whole, as before.
        assert_eq!(round_allow_target(&r, "Safari", Some("https://figma.com/file/1")), Some("figma.com".into()));
        assert_eq!(round_allow_target(&r, "Safari", None), None);
    }

    #[test]
    fn mark_as_work_moves_entries() {
        let mut r = Rules::default();
        mark_as_work(&mut r, &allow_target("Safari", Some("https://www.bilibili.com/x")).unwrap());
        assert!(!r.block_domains.iter().any(|d| d == "bilibili.com"));
        assert!(r.allow_domains.iter().any(|d| d == "bilibili.com"));
        mark_as_work(&mut r, "Steam");
        assert!(!r.block_apps.iter().any(|a| a == "Steam"));
        assert!(r.allow_apps.iter().any(|a| a == "Steam"));
        let s = sample("Safari", "x", Some("https://bilibili.com/"));
        assert!(evaluate(&r, &s).unwrap().0);
    }

    #[test]
    fn a_browser_is_never_the_allow_target() {
        assert_eq!(allow_target("Safari浏览器", None), None);
        assert_eq!(allow_target("Google Chrome", None), None);
        assert_eq!(allow_target("Arc", None), None);
        assert_eq!(allow_target("Helium", None), None);
        assert_eq!(allow_target("Google Chrome", Some("https://www.youtube.com/watch")), Some("youtube.com".into()));
        assert_eq!(allow_target("微信", None), Some("微信".into()));
        assert_eq!(allow_target("Archive Utility", None), Some("Archive Utility".into()));
    }

    #[test]
    fn bundle_name_matches() {
        let r = Rules::default();
        let mut s = sample("Electron", "main.ts", None);
        s.bundle_name = Some("Visual Studio Code".into());
        assert!(evaluate(&r, &s).unwrap().0);
    }
}
