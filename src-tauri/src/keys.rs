//! Where the API keys live: the macOS keychain, or for a debug build a folder of files.
//!
//! A debug build is signed afresh on every compile, and the keychain asks for the login
//! password each time a binary it hasn't seen reads an item. So a debug build keeps its keys
//! as plain files under the temp directory, apart from the installed app's, and only goes to
//! the keychain when POMO_KEYCHAIN=1 says so. A release build always uses the keychain.
use std::path::{Path, PathBuf};

const KEYCHAIN_SERVICE: &str = "PomoSupervisor";

/// The folder a debug build keeps its keys in; None when the keychain is the store.
fn dev_dir() -> Option<PathBuf> {
    let keychain = !cfg!(debug_assertions) || std::env::var("POMO_KEYCHAIN").is_ok_and(|v| !v.is_empty() && v != "0");
    (!keychain).then(|| std::env::temp_dir().join("pomosupervisor-dev").join("keys"))
}

/// The key saved for `slot` ("text" or "vision"), if there is one.
pub fn get(slot: &str) -> Option<String> {
    match dev_dir() {
        Some(dir) => read_file(&dir, slot),
        None => keyring::Entry::new(KEYCHAIN_SERVICE, slot).ok()?.get_password().ok().filter(|k| !k.is_empty()),
    }
}

/// Saves the key for `slot`; an empty key removes it.
pub fn set(slot: &str, key: &str) -> anyhow::Result<()> {
    if let Some(dir) = dev_dir() {
        return write_file(&dir, slot, key);
    }
    let entry = keyring::Entry::new(KEYCHAIN_SERVICE, slot)?;
    if key.is_empty() {
        let _ = entry.delete_credential();
    } else {
        entry.set_password(key)?;
    }
    Ok(())
}

fn read_file(dir: &Path, slot: &str) -> Option<String> {
    let key = std::fs::read_to_string(dir.join(slot)).ok()?;
    Some(key.trim().to_string()).filter(|k| !k.is_empty())
}

fn write_file(dir: &Path, slot: &str, key: &str) -> anyhow::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
    let path = dir.join(slot);
    if key.is_empty() {
        return match std::fs::remove_file(path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.into()),
            _ => Ok(()),
        };
    }
    // Plain text, so at least nobody else on the machine can read it.
    std::fs::DirBuilder::new().recursive(true).mode(0o700).create(dir)?;
    let mut file = std::fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(path)?;
    file.write_all(key.as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn a_debug_build_keeps_keys_in_files() {
        let dir = std::env::temp_dir().join(format!("pomo-keys-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(read_file(&dir, "text"), None);
        write_file(&dir, "text", "sk-one").unwrap();
        write_file(&dir, "text", "sk-two").unwrap();
        assert_eq!(read_file(&dir, "text").as_deref(), Some("sk-two"));
        assert_eq!(read_file(&dir, "vision"), None, "each slot has its own key");
        assert_eq!(std::fs::metadata(dir.join("text")).unwrap().permissions().mode() & 0o777, 0o600);
        // An empty key removes it, and removing what isn't there is fine.
        write_file(&dir, "text", "").unwrap();
        write_file(&dir, "text", "").unwrap();
        assert_eq!(read_file(&dir, "text"), None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
