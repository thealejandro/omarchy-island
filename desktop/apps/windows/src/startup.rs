//! Starting with Windows: a value under the user's own Run key (no admin,
//! nothing installed). It is only written when chosen from the tray.

use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::System::Registry::{
    HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_SZ, RegDeleteKeyValueW, RegGetValueW, RegSetKeyValueW,
};
use windows::core::{PCWSTR, w};

const RUN: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
const VALUE: PCWSTR = w!("Island");

/// The command the Run key should hold: this exe, quoted.
fn command() -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    Some(format!("\"{}\"", exe.display()))
}

/// On only if it starts this very exe (a moved one counts as off).
pub fn is_on() -> bool {
    let mut buf = [0u16; 1024];
    let mut size = size_of_val(&buf) as u32;
    let read = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            RUN,
            VALUE,
            RRF_RT_REG_SZ,
            None,
            Some(buf.as_mut_ptr().cast()),
            Some(&mut size),
        )
    };
    if read != ERROR_SUCCESS {
        return false;
    }
    let n = (size as usize / 2).min(buf.len());
    let got = String::from_utf16_lossy(&buf[..n]);
    command().is_some_and(|c| got.trim_end_matches('\0').eq_ignore_ascii_case(&c))
}

pub fn set(on: bool) -> windows::core::Result<()> {
    if !on {
        let e = unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, RUN, VALUE) };
        return if e == ERROR_SUCCESS || e.0 == 2 {
            // 2: ERROR_FILE_NOT_FOUND, already off.
            Ok(())
        } else {
            e.ok()
        };
    }
    let Some(cmd) = command() else {
        return Err(windows::core::Error::from_thread());
    };
    let data: Vec<u16> = cmd.encode_utf16().chain([0]).collect();
    unsafe {
        RegSetKeyValueW(
            HKEY_CURRENT_USER,
            RUN,
            VALUE,
            REG_SZ.0,
            Some(data.as_ptr().cast()),
            (data.len() * 2) as u32,
        )
    }
    .ok()
}
