//! Secret storage by account id: the OS secret store (Secret Service via `oo7` on
//! Linux, `keyring` on macOS/Windows). When it is unavailable (e.g. no Secret
//! Service agent running) secrets go to `<config dir>/secrets.toml` in plain text
//! and the Accounts page warns about it.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::features::settings::config_dir;

const SERVICE: &str = "desktop-rs";

/// Set once the secret store failed and the plain-text file is in use.
pub(super) static PLAINTEXT: AtomicBool = AtomicBool::new(false);

fn file_path() -> Option<PathBuf> {
    Some(config_dir()?.join("secrets.toml"))
}

/// Where secrets are kept in plain text, if that fallback is in use.
pub fn plaintext_path() -> Option<PathBuf> {
    PLAINTEXT.load(Ordering::Relaxed).then(file_path).flatten()
}

#[cfg(target_os = "linux")]
mod store {
    pub fn get(id: &str) -> Result<Option<String>, String> {
        smol::block_on(async {
            let keyring = oo7::Keyring::new().await.map_err(|e| e.to_string())?;
            let items = keyring.search_items(&[("service", super::SERVICE), ("id", id)]).await.map_err(|e| e.to_string())?;
            match items.first() {
                Some(item) => {
                    let secret = item.secret().await.map_err(|e| e.to_string())?;
                    Ok(Some(String::from_utf8_lossy(secret.as_bytes()).into_owned()))
                }
                None => Ok(None),
            }
        })
    }

    pub fn set(id: &str, secret: &str) -> Result<(), String> {
        smol::block_on(async {
            let keyring = oo7::Keyring::new().await.map_err(|e| e.to_string())?;
            let attributes = [("service", super::SERVICE), ("id", id)];
            keyring.create_item("desktop-rs account", &attributes, secret, true).await.map_err(|e| e.to_string())
        })
    }

    pub fn delete(id: &str) -> Result<(), String> {
        smol::block_on(async {
            let keyring = oo7::Keyring::new().await.map_err(|e| e.to_string())?;
            keyring.delete(&[("service", super::SERVICE), ("id", id)]).await.map_err(|e| e.to_string())
        })
    }
}

#[cfg(not(target_os = "linux"))]
mod store {
    fn entry(id: &str) -> Result<keyring::Entry, String> {
        keyring::Entry::new(super::SERVICE, id).map_err(|e| e.to_string())
    }

    pub fn get(id: &str) -> Result<Option<String>, String> {
        match entry(id)?.get_password() {
            Ok(s) => Ok(Some(s)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn set(id: &str, secret: &str) -> Result<(), String> {
        entry(id)?.set_password(secret).map_err(|e| e.to_string())
    }

    pub fn delete(id: &str) -> Result<(), String> {
        match entry(id)?.delete_credential() {
            Err(keyring::Error::NoEntry) => Ok(()),
            r => r.map_err(|e| e.to_string()),
        }
    }
}

fn read_file() -> BTreeMap<String, String> {
    let text = file_path().and_then(|p| std::fs::read_to_string(p).ok()).unwrap_or_default();
    toml::from_str(&text).unwrap_or_default()
}

fn write_file(map: &BTreeMap<String, String>) {
    let Some(path) = file_path() else { return };
    let text = toml::to_string(map).unwrap_or_default();
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    let res = std::fs::create_dir_all(path.parent().unwrap_or(&path))
        .and_then(|()| options.open(&path))
        .and_then(|mut f| std::io::Write::write_all(&mut f, text.as_bytes()));
    if let Err(e) = res {
        log::error!("cannot write {}: {e}", path.display());
    }
}

/// Switch to the plain-text file after the secret store failed.
fn fall_back(err: String) {
    if !PLAINTEXT.swap(true, Ordering::Relaxed) {
        log::warn!("secret store unavailable ({err}); secrets go to the plain-text file");
    }
}

pub fn get(id: &str) -> Option<String> {
    match store::get(id) {
        Ok(Some(s)) => Some(s),
        Ok(None) => read_file().remove(id),
        Err(e) => {
            fall_back(e);
            read_file().remove(id)
        }
    }
}

pub fn set(id: &str, secret: &str) {
    if let Err(e) = store::set(id, secret) {
        fall_back(e);
        let mut map = read_file();
        map.insert(id.into(), secret.into());
        write_file(&map);
    }
}

pub fn delete(id: &str) {
    if let Err(e) = store::delete(id) {
        fall_back(e);
    }
    let mut map = read_file();
    if map.remove(id).is_some() {
        write_file(&map);
    }
}
