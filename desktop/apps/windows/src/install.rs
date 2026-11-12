//! Installing the island for this user from its own exe (`--install`), and
//! taking it away again (`--uninstall`, what "Installed apps" runs). No
//! admin: everything goes under the user's own folders and keys; what goes
//! where is `island_core::install`. Settings and caches stay.

use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use island_core::install::{self, Layout, RegValue};
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, WAIT_OBJECT_0, WPARAM};
use windows::Win32::Storage::FileSystem::{
    GetFileVersionInfoSizeW, GetFileVersionInfoW, VS_FIXEDFILEINFO, VerQueryValueW,
};
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx, IPersistFile,
};
use windows::Win32::System::DataExchange::COPYDATASTRUCT;
use windows::Win32::System::Registry::{
    HKEY_CURRENT_USER, REG_DWORD, REG_SZ, RRF_RT_REG_SZ, RegDeleteTreeW, RegGetValueW,
    RegSetKeyValueW,
};
use windows::Win32::System::SystemInformation::GetSystemDirectoryW;
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_SYNCHRONIZE, PROCESS_TERMINATE, TerminateProcess, WaitForSingleObject,
};
use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowExW, GetWindowThreadProcessId, HWND_MESSAGE, PostMessageW, SMTO_ABORTIFHUNG,
    SendMessageTimeoutW, WM_COPYDATA,
};
use windows::core::{HSTRING, Interface, PCWSTR, w};

/// The installed island's first start: it says so with a glance.
pub const INSTALLED: &str = "--installed";
/// The copy in %TEMP% that takes the program's folder away once the
/// uninstaller has ended (a running exe can't be deleted).
const FINISH: &str = "--uninstall-finish";
/// No console window for the helper that deletes that copy.
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn layout() -> std::io::Result<Layout> {
    // Only full paths: an empty or relative one would land (and later
    // delete) under whatever the current folder is.
    let var = |k| {
        std::env::var_os(k)
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .ok_or_else(|| std::io::Error::other(k))
    };
    let (local, roaming) = (var("LOCALAPPDATA")?, var("APPDATA")?);
    Ok(install::layout(&local, &roaming))
}

/// Where an update found by "Check for updates" is downloaded:
/// `%LOCALAPPDATA%\island\updates`.
pub fn updates_dir() -> Option<PathBuf> {
    let local = PathBuf::from(std::env::var_os("LOCALAPPDATA")?);
    local
        .is_absolute()
        .then(|| local.join("island").join("updates"))
}

/// The downloaded updates go: once installed, or with the island.
pub fn clear_updates() {
    if let Some(dir) = updates_dir() {
        let _ = std::fs::remove_dir_all(dir);
    }
}

/// Whether `exe` is Island `version`, by its own resources.
pub fn is_island(exe: &Path, version: &str) -> bool {
    version_info(exe).is_some_and(|(v, product, company)| {
        v == version
            && product.as_deref() == Some("Island")
            && company.as_deref() == Some("thealejandro")
    })
}

/// The same file or folder, as the file system sees it (short 8.3 names,
/// redirected folders); as text when one isn't there yet.
fn same(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => install::same_path(a, b),
    }
}

/// A program of Windows' own, from its system folder: never one of the same
/// name beside this exe (Windows looks there first).
fn system_exe(name: &str) -> PathBuf {
    let mut buf = [0u16; 260];
    let n = unsafe { GetSystemDirectoryW(Some(&mut buf)) } as usize;
    PathBuf::from(String::from_utf16_lossy(&buf[..n.min(buf.len())])).join(name)
}

/// Takes away what the install put in its folder, then the folder if that
/// leaves it empty: nothing else that may be in there.
fn remove_program(l: &Layout) {
    let _ = std::fs::remove_file(&l.exe);
    let _ = std::fs::remove_dir(&l.dir);
}

fn other(e: windows::core::Error) -> std::io::Error {
    std::io::Error::other(e.message())
}

/// Ends the island that is running, if any: asked first with `quit`, ended
/// if it doesn't (an older one doesn't know the message).
pub fn close_running(anchor_class: PCWSTR, quit: u32) {
    unsafe {
        // An island from before the hidden window has only its own windows.
        let Ok(anchor) = FindWindowExW(Some(HWND_MESSAGE), None, anchor_class, None)
            .or_else(|_| FindWindowExW(None, None, w!("IslandWindow"), None))
        else {
            return;
        };
        let mut pid = 0;
        GetWindowThreadProcessId(anchor, Some(&mut pid));
        if pid == std::process::id() {
            return;
        }
        let Ok(p) = OpenProcess(PROCESS_SYNCHRONIZE | PROCESS_TERMINATE, false, pid) else {
            return;
        };
        let _ = PostMessageW(Some(anchor), quit, WPARAM(0), LPARAM(0));
        if WaitForSingleObject(p, 5000) != WAIT_OBJECT_0 {
            let _ = TerminateProcess(p, 0);
            let _ = WaitForSingleObject(p, 2000);
        }
        let _ = CloseHandle(p);
    }
}

/// The island to start again when installing fails, once the running one
/// was closed: the installed one, whole still, when this was an update;
/// else this exe.
pub fn back_after_failure() -> Option<PathBuf> {
    let me = std::env::current_exe().ok()?;
    match layout() {
        Ok(l) if l.exe.is_file() && !same(&me, &l.exe) => Some(l.exe),
        _ => Some(me),
    }
}

/// Whether this exe is the installed one.
pub fn is_installed() -> bool {
    match (std::env::current_exe(), layout()) {
        (Ok(me), Ok(l)) => same(&me, &l.exe),
        _ => false,
    }
}

/// Copies this exe to the user's programs, adds it to the Start menu and
/// "Installed apps", moves "Start with Windows" to it, and starts it.
pub fn install(anchor_class: PCWSTR, quit: u32) -> std::io::Result<()> {
    let l = layout()?;
    let me = std::env::current_exe()?;
    // The island that asked for this, ending by itself: until it has, its
    // exe is in use.
    let args: Vec<String> = std::env::args().collect();
    if let Some(pid) = arg_after(&args, WAIT_PID).and_then(|p| p.parse::<u32>().ok()) {
        wait_for(pid, 15_000);
    }
    close_running(anchor_class, quit);
    std::fs::create_dir_all(&l.dir)?;
    if !same(&me, &l.exe) {
        // Copied beside it first, then put in its place in one step: a copy
        // cut short (a full disk) never leaves a broken island installed.
        let new = l.exe.with_extension("exe.new");
        let put = || std::fs::copy(&me, &new).and_then(|_| std::fs::rename(&new, &l.exe));
        // Still in use for a moment (an antivirus scanning it, say).
        let mut tries = 0;
        while let Err(e) = put() {
            tries += 1;
            if tries == 40 {
                let _ = std::fs::remove_file(&new);
                return Err(e);
            }
            std::thread::sleep(std::time::Duration::from_millis(250));
        }
    }
    shortcut(&l).map_err(other)?;
    let size = std::fs::metadata(&l.exe).map_or(0, |m| m.len());
    for (name, v) in install::uninstall_values(&l.exe, env!("CARGO_PKG_VERSION"), size) {
        set_value(name, &v).map_err(other)?;
    }
    // On for the portable exe: on for the installed one (it would count as
    // off, and be lost, otherwise).
    if crate::startup::value().is_some_and(|v| install::run_points_elsewhere(&v, &l.exe)) {
        crate::startup::point_to(&l.exe).map_err(other)?;
    }
    Command::new(&l.exe).arg(INSTALLED).spawn()?;
    Ok(())
}

fn shortcut(l: &Layout) -> windows::core::Result<()> {
    if let Some(dir) = l.shortcut.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)?;
        link.SetPath(&HSTRING::from(l.exe.as_path()))?;
        link.SetWorkingDirectory(&HSTRING::from(l.dir.as_path()))?;
        link.SetIconLocation(&HSTRING::from(l.exe.as_path()), 0)?;
        link.SetDescription(w!("Island"))?;
        link.cast::<IPersistFile>()?
            .Save(&HSTRING::from(l.shortcut.as_path()), true)
    }
}

fn set_value(name: &str, v: &RegValue) -> windows::core::Result<()> {
    let key = HSTRING::from(install::UNINSTALL_KEY);
    let name = HSTRING::from(name);
    let e = unsafe {
        match v {
            RegValue::Str(s) => {
                let d: Vec<u16> = s.encode_utf16().chain([0]).collect();
                RegSetKeyValueW(
                    HKEY_CURRENT_USER,
                    &key,
                    &name,
                    REG_SZ.0,
                    Some(d.as_ptr().cast()),
                    (d.len() * 2) as u32,
                )
            }
            RegValue::Dword(n) => RegSetKeyValueW(
                HKEY_CURRENT_USER,
                &key,
                &name,
                REG_DWORD.0,
                Some((n as *const u32).cast()),
                4,
            ),
        }
    };
    e.ok()
}

/// Windows' banners back on (before anything goes: it needs the notes),
/// then the Run value, the shortcut, the entry and the program's folder.
pub fn uninstall(anchor_class: PCWSTR, quit: u32) -> std::io::Result<()> {
    let l = layout()?;
    close_running(anchor_class, quit);
    crate::banners::restore();
    clear_updates();
    // Each step goes on if one before failed: half an uninstall would be
    // worse. The first failure is told at the end.
    let mut failed = crate::startup::set(false).err().map(other);
    let _ = std::fs::remove_file(&l.shortcut);
    unsafe {
        let _ = RegDeleteTreeW(HKEY_CURRENT_USER, &HSTRING::from(install::UNINSTALL_KEY));
    }
    let me = std::env::current_exe()?;
    let inside = me.parent().is_some_and(|p| same(p, &l.dir));
    if !inside {
        remove_program(&l);
    } else if let Err(e) = start_finish(&me) {
        failed.get_or_insert(e);
    }
    failed.map_or(Ok(()), Err)
}

/// This exe is in the folder: a copy in a new folder of its own under
/// %TEMP% (no name known beforehand, nothing else beside it to load) waits
/// for it to end.
fn start_finish(me: &Path) -> std::io::Result<()> {
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos())
        ^ (std::process::id() as u128) << 64;
    let dir = std::env::temp_dir().join(format!("island-uninstall-{seed:x}"));
    // Fails if it is there already: never a folder someone else made.
    std::fs::create_dir(&dir)?;
    let copy = dir.join("island.exe");
    std::fs::copy(me, &copy)?;
    Command::new(&copy)
        .arg(FINISH)
        .arg(std::process::id().to_string())
        .current_dir(&dir)
        .spawn()?;
    Ok(())
}

/// The `--uninstall-finish <pid>` copy: once the uninstaller `pid` has
/// ended, the program's folder goes (always the one `layout` says, never one
/// from the command line), and then this copy.
pub fn finish(args: &[String]) -> bool {
    let Some(pid) = arg_after(args, FINISH).and_then(|p| p.parse::<u32>().ok()) else {
        return false;
    };
    wait_for(pid, 10_000);
    if let Ok(l) = layout() {
        for _ in 0..10 {
            remove_program(&l);
            if !l.exe.exists() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(300));
        }
    }
    if let Ok(me) = std::env::current_exe()
        && let Some(dir) = me.parent()
    {
        // This copy and its folder, by Windows' own cmd once it has ended.
        let _ = Command::new(system_exe("cmd.exe"))
            .arg("/c")
            .raw_arg(format!(
                "ping -n 3 127.0.0.1 >nul & del /q \"{}\" & rmdir \"{}\"",
                me.display(),
                dir.display()
            ))
            .current_dir(system_exe(""))
            .creation_flags(CREATE_NO_WINDOW)
            .spawn();
    }
    true
}

/// The version "Installed apps" lists, if any.
fn installed_version() -> Option<String> {
    let mut buf = [0u16; 64];
    let mut len = (buf.len() * 2) as u32;
    let e = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            &HSTRING::from(install::UNINSTALL_KEY),
            w!("DisplayVersion"),
            RRF_RT_REG_SZ,
            None,
            Some(buf.as_mut_ptr().cast()),
            Some(&mut len),
        )
    };
    if e.is_err() {
        return None;
    }
    let n = (len as usize / 2).min(buf.len());
    let v = String::from_utf16_lossy(&buf[..n]);
    Some(v.trim_end_matches('\0').to_owned())
}

/// `%LOCALAPPDATA%\island\portable.txt`: the version the user chose to use
/// without installing.
fn portable_file() -> Option<PathBuf> {
    let local = PathBuf::from(std::env::var_os("LOCALAPPDATA")?);
    local
        .is_absolute()
        .then(|| local.join("island").join("portable.txt"))
}

/// What this exe offers at start: to install itself, or to update an older
/// installed one.
pub fn offer() -> Option<install::Offer> {
    let l = layout().ok()?;
    let me = std::env::current_exe().ok()?;
    let declined = portable_file().and_then(|p| std::fs::read_to_string(p).ok());
    install::offer(
        same(&me, &l.exe),
        installed_version().as_deref(),
        env!("CARGO_PKG_VERSION"),
        declined.as_deref(),
    )
}

/// "Use without installing": not offered again for `version`.
pub fn decline(version: &str) {
    if let Some(p) = portable_file() {
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(p, version);
    }
}

/// Installs `exe` in a process of its own (it closes the running island
/// first).
pub fn start_install(exe: &Path) -> std::io::Result<()> {
    Command::new(exe)
        .arg("--install")
        .arg(WAIT_PID)
        .arg(std::process::id().to_string())
        .spawn()?;
    Ok(())
}

/// `--wait-pid <pid>`: the island that started the install, which quits.
const WAIT_PID: &str = "--wait-pid";

/// The argument after `name`.
fn arg_after<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter()
        .skip_while(|a| *a != name)
        .nth(1)
        .map(String::as_str)
}

/// Until process `pid` has ended, `ms` at most.
fn wait_for(pid: u32, ms: u32) {
    unsafe {
        if let Ok(p) = OpenProcess(PROCESS_SYNCHRONIZE, false, pid) {
            let _ = WaitForSingleObject(p, ms);
            let _ = CloseHandle(p);
        }
    }
}

/// The install failed: the exe it came from starts again and says so.
pub const INSTALL_FAILED: &str = "--install-failed";

/// Marks the `WM_COPYDATA` a newer exe sends the running island ("ISL1").
const OFFER_DATA: usize = 0x4953_4C31;

/// The running island's answer to `send_offer`, as `WM_COPYDATA`'s result.
pub const OFFER_SHOWN: isize = 1;
/// It knows the message but won't offer this exe (not newer than itself, or
/// its menu couldn't open): it stays.
pub const OFFER_DECLINED: isize = 2;

/// What the running island did with `send_offer`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reply {
    Shown,
    /// It answered no, or didn't answer in time: it stays as it is.
    Declined,
    /// One from before this message (it answers 0): only by taking its
    /// place can the update be offered.
    Unknown,
}

/// Asks the island running (`anchor`) to offer the update to this exe, so
/// it keeps running until the user chooses.
pub fn send_offer(anchor: HWND) -> Reply {
    let Ok(me) = std::env::current_exe() else {
        return Reply::Declined;
    };
    let text = install::offer_text(env!("CARGO_PKG_VERSION"), &me.to_string_lossy());
    let data: Vec<u16> = text.encode_utf16().collect();
    let cds = COPYDATASTRUCT {
        dwData: OFFER_DATA,
        cbData: (data.len() * 2) as u32,
        lpData: data.as_ptr() as *mut _,
    };
    let mut out = 0usize;
    let sent = unsafe {
        SendMessageTimeoutW(
            anchor,
            WM_COPYDATA,
            WPARAM(0),
            LPARAM(&cds as *const _ as isize),
            SMTO_ABORTIFHUNG,
            3000,
            Some(&mut out),
        )
    };
    match (sent.0 != 0, out as isize) {
        (true, OFFER_SHOWN) => Reply::Shown,
        (true, 0) => Reply::Unknown,
        _ => Reply::Declined,
    }
}

/// The newer exe and its version in a `send_offer`, if it is one: a full
/// path to a file that is there, newer than this island.
///
/// # Safety
/// `lp` is a `WM_COPYDATA`'s.
pub unsafe fn read_offer(lp: LPARAM) -> Option<(String, PathBuf)> {
    let cds = unsafe { (lp.0 as *const COPYDATASTRUCT).as_ref()? };
    let n = cds.cbData as usize / 2;
    if cds.dwData != OFFER_DATA || cds.lpData.is_null() || n > 4096 {
        return None;
    }
    let data = unsafe { std::slice::from_raw_parts(cds.lpData as *const u16, n) };
    let text = String::from_utf16(data).ok()?;
    let (_, exe) = install::read_offer_text(&text)?;
    let exe = PathBuf::from(exe);
    if !exe.is_absolute() || !exe.is_file() {
        return None;
    }
    // The exe's own version, never the one the message says; and only an
    // Island's.
    let (version, product, company) = version_info(&exe)?;
    let island = product.as_deref() == Some("Island") && company.as_deref() == Some("thealejandro");
    (island && install::newer(&version, env!("CARGO_PKG_VERSION"))).then_some((version, exe))
}

/// An exe's version (`a.b.c`), product and company, from its resources (as
/// build.rs writes Island's).
fn version_info(exe: &Path) -> Option<(String, Option<String>, Option<String>)> {
    let name = HSTRING::from(exe);
    unsafe {
        let size = GetFileVersionInfoSizeW(&name, None);
        if size == 0 {
            return None;
        }
        let mut data = vec![0u8; size as usize];
        GetFileVersionInfoW(&name, None, size, data.as_mut_ptr().cast()).ok()?;
        let query = |path: PCWSTR| {
            let mut out: *mut std::ffi::c_void = std::ptr::null_mut();
            let mut len = 0u32;
            let found = VerQueryValueW(data.as_ptr().cast(), path, &mut out, &mut len).as_bool();
            (found && !out.is_null()).then_some((out, len as usize))
        };
        let text = |path: PCWSTR| {
            let (p, n) = query(path)?;
            let s = std::slice::from_raw_parts(p as *const u16, n);
            Some(
                String::from_utf16_lossy(s)
                    .trim_end_matches('\0')
                    .to_owned(),
            )
        };
        let (info, len) = query(w!("\\"))?;
        if len < size_of::<VS_FIXEDFILEINFO>() {
            return None;
        }
        let f = &*(info as *const VS_FIXEDFILEINFO);
        let version = format!(
            "{}.{}.{}",
            f.dwFileVersionMS >> 16,
            f.dwFileVersionMS & 0xFFFF,
            f.dwFileVersionLS >> 16
        );
        Some((
            version,
            text(w!("\\StringFileInfo\\040904B0\\ProductName")),
            text(w!("\\StringFileInfo\\040904B0\\CompanyName")),
        ))
    }
}

/// Whether `arg` is on the command line.
pub fn has(args: &[String], arg: &str) -> bool {
    args.iter().any(|a| a == arg)
}

pub fn is_finish(args: &[String]) -> bool {
    has(args, FINISH)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_an_exes_own_version_and_product() {
        let (v, product, _) = version_info(&system_exe("notepad.exe")).expect("notepad has one");
        assert_eq!(v.split('.').count(), 3);
        assert!(v.split('.').all(|p| p.parse::<u32>().is_ok()));
        // Not an Island: never offered, whatever its version.
        assert_ne!(product.as_deref(), Some("Island"));
        assert_eq!(version_info(&system_exe("no-such-island.exe")), None);
        // This one is built with Island's resources (build.rs).
        let me = std::env::current_exe().unwrap();
        let (v, product, company) = version_info(&me).expect("built with a version");
        assert_eq!(v, env!("CARGO_PKG_VERSION"));
        assert_eq!(product.as_deref(), Some("Island"));
        assert_eq!(company.as_deref(), Some("thealejandro"));
    }
}
