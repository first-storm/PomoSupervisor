//! Text from the frontmost window through the macOS Accessibility API, for the text model.
//! Read only when the rules had no answer; nothing here is stored.

#[derive(Debug, Clone, Default, PartialEq)]
pub struct AxInfo {
    /// The focused control: role, label and (clipped) value.
    pub focused: Option<String>,
    /// Visible text of the focused window, de-duplicated and clipped.
    pub text: Option<String>,
    /// `AXURL` of a web area, for browsers AppleScript can't ask (Firefox).
    pub url: Option<String>,
}

/// Most characters of window text sent to the model.
pub const MAX_TEXT_CHARS: usize = 800;
const MAX_LINE_CHARS: usize = 200;

fn clip(s: &str, n: usize) -> String {
    match s.char_indices().nth(n) {
        Some((i, _)) => format!("{}…", &s[..i]),
        None => s.to_string(),
    }
}

fn squash(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Whitespace folded, blanks and one-character lines dropped, repeats removed, clipped to `max_chars`.
pub fn summarize(lines: impl IntoIterator<Item = String>, max_chars: usize) -> Option<String> {
    let mut seen = std::collections::HashSet::new();
    let mut out = String::new();
    let mut used = 0;
    for line in lines {
        let line = clip(&squash(&line), MAX_LINE_CHARS);
        if line.chars().count() < 2 || !seen.insert(line.clone()) {
            continue;
        }
        let left = max_chars.saturating_sub(used);
        if left == 0 {
            break;
        }
        let line = clip(&line, left);
        used += line.chars().count() + 1;
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&line);
    }
    (!out.is_empty()).then_some(out)
}

/// A cache key part that ignores digits, so clocks, progress and unread counts don't defeat the cache.
pub fn fingerprint(ax: &AxInfo) -> String {
    let text: String = ax.text.as_deref().unwrap_or("").chars().take(120).collect();
    format!("{}\u{1}{}", ax.focused.as_deref().unwrap_or(""), text)
        .chars()
        .filter(|c| !c.is_ascii_digit())
        .collect()
}

#[cfg(target_os = "macos")]
mod mac {
    use super::*;
    use core_foundation::array::{CFArray, CFArrayGetTypeID};
    use core_foundation::base::{CFGetTypeID, CFRange, CFType, CFTypeRef, TCFType};
    use core_foundation::boolean::CFBoolean;
    use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
    use core_foundation::string::{CFString, CFStringGetCharacters, CFStringGetLength, CFStringRef};
    use core_foundation::url::CFURL;
    use std::collections::HashSet;
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    #[link(name = "ApplicationServices", kind = "framework")]
    unsafe extern "C" {
        fn AXIsProcessTrusted() -> bool;
        fn AXIsProcessTrustedWithOptions(options: CFDictionaryRef) -> bool;
        static kAXTrustedCheckOptionPrompt: CFStringRef;
        fn AXUIElementCreateApplication(pid: i32) -> CFTypeRef;
        fn AXUIElementCopyAttributeValue(el: CFTypeRef, attr: CFStringRef, value: *mut CFTypeRef) -> i32;
        fn AXUIElementSetAttributeValue(el: CFTypeRef, attr: CFStringRef, value: CFTypeRef) -> i32;
        fn AXUIElementSetMessagingTimeout(el: CFTypeRef, secs: f32) -> i32;
    }

    const MAX_NODES: usize = 800;
    const MAX_DEPTH: usize = 40;
    const BUDGET: Duration = Duration::from_millis(400);
    /// Roles whose title / value / description is worth reading. Buttons are left out: mostly chrome.
    const TEXT_ROLES: &[&str] = &[
        "AXStaticText", "AXHeading", "AXTextField", "AXTextArea", "AXLink", "AXTab", "AXCell", "AXRow", "AXImage",
        "AXGroup", "AXComboBox", "AXCheckBox", "AXRadioButton", "AXMenuButton", "AXPopUpButton", "AXDocument",
    ];
    /// Subtrees skipped: browser and app chrome, sidebars, password fields.
    fn skip(el: &CFType, role: &str, sub: &str) -> bool {
        matches!(role, "AXSecureTextField" | "AXToolbar" | "AXMenuBar" | "AXScrollBar")
            || matches!(sub, "AXSourceList" | "AXLandmarkNavigation" | "AXLandmarkBanner" | "AXLandmarkContentInfo")
            // Finder, Mail, Notes…: the sidebar is an outline that says so.
            || (role == "AXOutline"
                && string(el, "AXDescription").is_some_and(|d| matches!(d.to_lowercase().as_str(), "sidebar" | "侧边栏" | "边栏")))
    }
    /// Gecko browsers build their tree only for an assistive technology, which `AXEnhancedUserInterface` stands in for.
    /// Other apps don't get it: it can disturb window animations.
    fn wants_enhanced(app: &str) -> bool {
        let a = app.to_lowercase();
        a.starts_with("firefox") || matches!(a.as_str(), "librewolf" | "waterfox" | "floorp" | "zen" | "zen browser" | "tor browser")
    }

    /// Apps already asked to expose their content.
    static ASKED: Mutex<Option<HashSet<i32>>> = Mutex::new(None);

    pub fn trusted() -> bool {
        unsafe { AXIsProcessTrusted() }
    }

    pub fn request() -> bool {
        let key = unsafe { CFString::wrap_under_get_rule(kAXTrustedCheckOptionPrompt) };
        let opts = CFDictionary::from_CFType_pairs(&[(key.as_CFType(), CFBoolean::true_value().as_CFType())]);
        unsafe { AXIsProcessTrustedWithOptions(opts.as_concrete_TypeRef()) }
    }

    fn attr(el: &CFType, name: &str) -> Option<CFType> {
        let name = CFString::new(name);
        let mut v: CFTypeRef = std::ptr::null();
        let err = unsafe { AXUIElementCopyAttributeValue(el.as_CFTypeRef(), name.as_concrete_TypeRef(), &mut v) };
        (err == 0 && !v.is_null()).then(|| unsafe { CFType::wrap_under_create_rule(v) })
    }

    /// `CFString::to_string` asserts the whole string converts to UTF-8 and panics when it doesn't
    /// (an unpaired surrogate; Telegram's window has such a string). This replaces bad units with U+FFFD.
    pub(super) fn lossy(s: &CFString) -> String {
        let len = unsafe { CFStringGetLength(s.as_concrete_TypeRef()) };
        if len <= 0 {
            return String::new();
        }
        let mut buf = vec![0u16; len as usize];
        unsafe { CFStringGetCharacters(s.as_concrete_TypeRef(), CFRange { location: 0, length: len }, buf.as_mut_ptr()) };
        String::from_utf16_lossy(&buf)
    }

    fn string(el: &CFType, name: &str) -> Option<String> {
        let s = lossy(&attr(el, name)?.downcast::<CFString>()?);
        let s = s.trim();
        (!s.is_empty()).then(|| s.to_string())
    }

    fn children(el: &CFType) -> Vec<CFType> {
        let Some(v) = attr(el, "AXChildren") else { return vec![] };
        if unsafe { CFGetTypeID(v.as_CFTypeRef()) != CFArrayGetTypeID() } {
            return vec![];
        }
        let arr: CFArray<CFType> = unsafe { CFArray::wrap_under_get_rule(v.as_CFTypeRef() as _) };
        arr.iter().map(|c| c.clone()).collect()
    }

    fn url_of(el: &CFType) -> Option<String> {
        let v = attr(el, "AXURL")?;
        if let Some(u) = v.downcast::<CFURL>() {
            return Some(lossy(&u.get_string()));
        }
        v.downcast::<CFString>().map(|s| lossy(&s))
    }

    /// Title, description and string value of one element, without repeats.
    fn labels(el: &CFType) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for k in ["AXTitle", "AXDescription", "AXValue"] {
            if let Some(s) = string(el, k)
                && !out.contains(&s)
            {
                out.push(s);
            }
        }
        out
    }

    /// Chromium / Electron expose web content after `AXManualAccessibility`; Gecko after `AXEnhancedUserInterface`.
    fn ask_for_content(app: &CFType, pid: i32, name: &str) {
        let mut set = ASKED.lock().unwrap();
        if !set.get_or_insert_with(HashSet::new).insert(pid) {
            return;
        }
        let mut attrs = vec!["AXManualAccessibility"];
        if wants_enhanced(name) {
            attrs.push("AXEnhancedUserInterface");
        }
        for a in attrs {
            let a = CFString::new(a);
            unsafe {
                AXUIElementSetAttributeValue(app.as_CFTypeRef(), a.as_concrete_TypeRef(), CFBoolean::true_value().as_CFTypeRef());
            }
        }
    }

    /// The first element with this role (and subrole, if given) under `root`, breadth-first.
    fn find(root: &CFType, role: &str, sub: Option<&str>, limit: usize, deadline: Instant) -> Option<CFType> {
        let mut queue = std::collections::VecDeque::from([root.clone()]);
        let mut n = 0;
        while let Some(el) = queue.pop_front() {
            n += 1;
            if n > limit || Instant::now() > deadline {
                return None;
            }
            if string(&el, "AXRole").as_deref() == Some(role) && sub.is_none_or(|s| string(&el, "AXSubrole").as_deref() == Some(s)) {
                return Some(el);
            }
            if !string(&el, "AXRole").is_some_and(|r| r == "AXToolbar") {
                queue.extend(children(&el));
            }
        }
        None
    }

    /// Text under `root` in reading order (depth-first), within the shared node and time budget.
    fn collect(root: &CFType, lines: &mut Vec<String>, nodes: &mut usize, deadline: Instant) {
        let mut stack = vec![(root.clone(), 0usize)];
        while let Some((el, depth)) = stack.pop() {
            *nodes += 1;
            if *nodes > MAX_NODES || Instant::now() > deadline {
                return;
            }
            let role = string(&el, "AXRole").unwrap_or_default();
            let sub = string(&el, "AXSubrole").unwrap_or_default();
            if skip(&el, &role, &sub) {
                continue;
            }
            if role == "AXImage" {
                // Icon names ("clock", "photo") are noise; a title is a file name or a caption.
                lines.extend(string(&el, "AXTitle"));
            } else if TEXT_ROLES.contains(&role.as_str()) {
                lines.extend(labels(&el));
            }
            if depth < MAX_DEPTH {
                stack.extend(children(&el).into_iter().rev().map(|k| (k, depth + 1)));
            }
        }
    }

    /// Roles and labels of the focused window's tree, for `examples/ax_dump`.
    #[doc(hidden)]
    pub fn debug_tree(pid: i32, max_depth: usize) -> String {
        let app = unsafe { CFType::wrap_under_create_rule(AXUIElementCreateApplication(pid)) };
        ask_for_content(&app, pid, "");
        let Some(win) = attr(&app, "AXFocusedWindow") else { return "no focused window".into() };
        let mut out = String::new();
        let mut stack = vec![(win, 0usize)];
        let mut n = 0;
        while let Some((el, d)) = stack.pop() {
            n += 1;
            if n > 300 {
                break;
            }
            let kids = children(&el);
            let role = string(&el, "AXRole").unwrap_or_default();
            let sub = string(&el, "AXSubrole").unwrap_or_default();
            let lab = labels(&el).iter().map(|s| clip(&squash(s), 40)).collect::<Vec<_>>().join(" | ");
            out.push_str(&format!("{}{role}/{sub} [{}] {lab}\n", "  ".repeat(d), kids.len()));
            if d < max_depth {
                stack.extend(kids.into_iter().rev().map(|k| (k, d + 1)));
            }
        }
        out
    }

    /// `name` is the app's name, to pick how to ask it for its content.
    pub fn read(pid: i32, name: &str) -> Option<AxInfo> {
        if !trusted() {
            return None;
        }
        let deadline = Instant::now() + BUDGET;
        let app = unsafe {
            let r = AXUIElementCreateApplication(pid);
            if r.is_null() {
                return None;
            }
            CFType::wrap_under_create_rule(r)
        };
        unsafe { AXUIElementSetMessagingTimeout(app.as_CFTypeRef(), 0.25) };
        ask_for_content(&app, pid, name);

        let focused = attr(&app, "AXFocusedUIElement").and_then(|f| {
            let role = string(&f, "AXRole")?;
            if role == "AXSecureTextField" {
                return Some(role);
            }
            let parts = labels(&f).into_iter().map(|s| clip(&squash(&s), MAX_LINE_CHARS)).collect::<Vec<_>>();
            Some(format!("{role} {}", parts.join(" · ")).trim().to_string())
        });

        let mut lines = Vec::new();
        let mut url = None;
        if let Some(win) = attr(&app, "AXFocusedWindow") {
            if let Some(doc) = string(&win, "AXDocument") {
                let doc = url::Url::parse(&doc)
                    .ok()
                    .and_then(|u| u.to_file_path().ok())
                    .map(|p| p.display().to_string())
                    .unwrap_or(doc);
                lines.push(format!("文档：{doc}"));
            }
            // The page's main content first, then the rest of the page, then the window; repeats drop out later.
            let mut roots = Vec::new();
            if let Some(web) = find(&win, "AXWebArea", None, 300, deadline) {
                url = url_of(&web).filter(|u| u.starts_with("http"));
                if let Some(main) = find(&web, "AXGroup", Some("AXLandmarkMain"), 300, deadline) {
                    roots.push(main);
                }
                roots.push(web);
            }
            roots.push(win);
            let mut nodes = 0;
            for r in &roots {
                collect(r, &mut lines, &mut nodes, deadline);
            }
        }
        Some(AxInfo { focused, text: summarize(lines, MAX_TEXT_CHARS), url })
    }
}

#[cfg(target_os = "macos")]
pub use mac::{debug_tree, read, request, trusted};

#[cfg(not(target_os = "macos"))]
pub fn trusted() -> bool {
    false
}
#[cfg(not(target_os = "macos"))]
pub fn request() -> bool {
    false
}
#[cfg(not(target_os = "macos"))]
pub fn read(_pid: i32, _name: &str) -> Option<AxInfo> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summarize_folds_dedups_and_clips_by_char() {
        let lines = vec!["  你好   世界 ".to_string(), "你好 世界".into(), "x".into(), "".into(), "第二行".into()];
        assert_eq!(summarize(lines, 100).as_deref(), Some("你好 世界\n第二行"));
        let long = vec!["汉".repeat(50), "字".repeat(50)];
        let s = summarize(long, 60).unwrap();
        assert!(s.chars().count() <= 62, "{}", s.chars().count());
        assert!(s.starts_with(&"汉".repeat(50)));
        assert_eq!(summarize(Vec::<String>::new(), 100), None);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn unpaired_surrogate_does_not_panic() {
        use core_foundation::base::{TCFType, kCFAllocatorDefault};
        use core_foundation::string::{CFString, CFStringCreateWithCharacters};
        // "hi" + a lone high surrogate, as in a preview clipped in the middle of an emoji.
        let units = [0x68u16, 0x69, 0xD83D];
        let s = unsafe { CFString::wrap_under_create_rule(CFStringCreateWithCharacters(kCFAllocatorDefault, units.as_ptr(), 3)) };
        assert!(std::panic::catch_unwind(|| s.to_string()).is_err(), "the crate's conversion panics on this");
        assert_eq!(mac::lossy(&s), "hi\u{FFFD}");
        assert_eq!(mac::lossy(&CFString::new("你好 😀")), "你好 😀");
    }

    #[test]
    fn fingerprint_ignores_digits() {
        let a = AxInfo { focused: Some("AXTextArea 消息".into()), text: Some("未读 3\n12:01".into()), url: None };
        let b = AxInfo { focused: Some("AXTextArea 消息".into()), text: Some("未读 15\n12:02".into()), url: None };
        let c = AxInfo { focused: Some("AXTextArea 消息".into()), text: Some("另一个聊天".into()), url: None };
        assert_eq!(fingerprint(&a), fingerprint(&b));
        assert_ne!(fingerprint(&a), fingerprint(&c));
    }
}
