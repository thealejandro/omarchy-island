//! "Check for updates": only when chosen in the menu, a thread of its own
//! asks GitHub Releases for the newest island, downloads it and holds it to
//! its release's SHA-256 before it is offered. Nothing runs in between.
//!
//! The checksum comes from the same release, so it proves the download
//! whole, not who made it: with no signing, HTTPS to github.com is what is
//! trusted. The exe must also say it is Island, of that version.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use island_core::update::{self, Version};
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::Security::Cryptography::{BCRYPT_SHA256_ALG_HANDLE, BCryptHash};
use windows::Win32::UI::WindowsAndMessaging::PostMessageW;

use crate::http::{self, NetError};

/// What a check found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    UpToDate,
    Offline,
    /// GitHub said no (rate limit, an error) or the files didn't hold up.
    Failed,
    /// Newer, for a copy that doesn't update itself (not the installed one).
    Available(Version),
    /// Newer, downloaded and checked: this exe.
    Ready(Version, PathBuf),
}

#[derive(Default)]
pub struct Updater {
    busy: Arc<AtomicBool>,
    result: Arc<Mutex<Option<Outcome>>>,
}

impl Updater {
    /// A check is under way.
    pub fn busy(&self) -> bool {
        self.busy.load(Ordering::Acquire)
    }

    /// Starts a check (none if one is under way); `msg` goes to `hwnd` when
    /// it ends. `download` for the installed copy, which updates itself.
    pub fn check(&self, hwnd: HWND, msg: u32, download: bool) {
        if self.busy.swap(true, Ordering::AcqRel) {
            return;
        }
        let (busy, result) = (self.busy.clone(), self.result.clone());
        let hwnd = hwnd.0 as isize;
        let started = std::thread::Builder::new()
            .name("update".into())
            .spawn(move || {
                let found = run(download);
                if let Ok(mut r) = result.lock() {
                    *r = Some(found);
                }
                busy.store(false, Ordering::Release);
                unsafe {
                    let _ = PostMessageW(Some(HWND(hwnd as *mut _)), msg, WPARAM(0), LPARAM(0));
                }
            });
        if started.is_err() {
            self.busy.store(false, Ordering::Release);
        }
    }

    /// What the last check found, once.
    pub fn take(&self) -> Option<Outcome> {
        self.result.lock().ok()?.take()
    }
}

fn running() -> Option<Version> {
    Version::parse(env!("CARGO_PKG_VERSION"))
}

fn failed(e: NetError) -> Outcome {
    match e {
        NetError::Offline => Outcome::Offline,
        NetError::Failed => Outcome::Failed,
    }
}

fn run(download: bool) -> Outcome {
    let Some(me) = running() else {
        return Outcome::Failed;
    };
    let (code, body) = match http::get(update::API_HOST, update::RELEASES_PATH, update::MAX_LIST) {
        Ok(r) => r,
        Err(e) => return failed(e),
    };
    if code != 200 {
        return Outcome::Failed;
    }
    let Ok(list) = serde_json::from_slice(&body) else {
        return Outcome::Failed;
    };
    let Some(release) = update::pick(&list, me) else {
        return Outcome::UpToDate;
    };
    if !download {
        return Outcome::Available(release.version);
    }
    fetch(&release).map_or(Outcome::Failed, |exe| Outcome::Ready(release.version, exe))
}

/// The release's exe, downloaded and checked, in [`crate::install::updates_dir`].
fn fetch(release: &update::Release) -> Option<PathBuf> {
    let name = update::asset_name(release.version);
    let get = |file: &str, cap: usize| {
        let path = update::download_path(&release.tag, file);
        match http::get(update::HOST, &path, cap) {
            Ok((200, body)) => Some(body),
            _ => None,
        }
    };
    let sums = get(update::SUMS, update::MAX_SUMS)?;
    let want = update::sums_hash(&String::from_utf8_lossy(&sums), &name)?;
    let exe = get(&name, update::MAX_EXE as usize)?;
    if sha256(&exe)? != want {
        return None;
    }
    let dir = crate::install::updates_dir()?;
    std::fs::create_dir_all(&dir).ok()?;
    let path = dir.join(&name);
    let part = path.with_extension("exe.part");
    std::fs::write(&part, &exe).ok()?;
    std::fs::rename(&part, &path).ok()?;
    if !crate::install::is_island(&path, &release.version.to_string()) {
        let _ = std::fs::remove_file(&path);
        return None;
    }
    Some(path)
}

fn sha256(data: &[u8]) -> Option<[u8; 32]> {
    let mut out = [0u8; 32];
    unsafe { BCryptHash(BCRYPT_SHA256_ALG_HANDLE, None, data, &mut out) }
        .is_ok()
        .then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_as_sha256() {
        let abc = sha256(b"abc").unwrap();
        let hex: String = abc.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(
            hex,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn its_own_version_is_one() {
        assert!(running().is_some());
    }
}
