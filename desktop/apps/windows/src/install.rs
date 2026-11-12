//! Installing the island for this user from its own exe (`--install`), and
//! taking it away again (`--uninstall`, what "Installed apps" runs). No
//! admin: everything goes under the user's own folders and keys; what goes
//! where is `island_core::install`. Settings and caches stay.

use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use island_core::install::{self, Layout, RegValue};
use windows::Win32::Foundation::{CloseHandle, LPARAM, WAIT_OBJECT_0, WPARAM};
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx, IPersistFile,
};
use windows::Win32::System::Registry::{
    HKEY_CURRENT_USER, REG_DWORD, REG_SZ, RegDeleteTreeW, RegSetKeyValueW,
};
use windows::Win32::System::SystemInformation::GetSystemDirectoryW;
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_SYNCHRONIZE, PROCESS_TERMINATE, TerminateProcess, WaitForSingleObject,
};
use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowExW, GetWindowThreadProcessId, HWND_MESSAGE, PostMessageW,
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

/// Copies this exe to the user's programs, adds it to the Start menu and
/// "Installed apps", moves "Start with Windows" to it, and starts it.
pub fn install(anchor_class: PCWSTR, quit: u32) -> std::io::Result<()> {
    let l = layout()?;
    let me = std::env::current_exe()?;
    close_running(anchor_class, quit);
    std::fs::create_dir_all(&l.dir)?;
    if !same(&me, &l.exe) {
        std::fs::copy(&me, &l.exe)?;
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
    let Some(pid) = args
        .iter()
        .skip_while(|a| *a != FINISH)
        .nth(1)
        .and_then(|p| p.parse::<u32>().ok())
    else {
        return false;
    };
    unsafe {
        if let Ok(p) = OpenProcess(PROCESS_SYNCHRONIZE, false, pid) {
            let _ = WaitForSingleObject(p, 10_000);
            let _ = CloseHandle(p);
        }
    }
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

/// Whether `arg` is on the command line.
pub fn has(args: &[String], arg: &str) -> bool {
    args.iter().any(|a| a == arg)
}

pub fn is_finish(args: &[String]) -> bool {
    has(args, FINISH)
}
