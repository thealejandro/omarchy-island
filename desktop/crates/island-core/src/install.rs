//! Where a per-user install puts the island and what the registry would
//! say about it: pure planning, so the app does the actual installing and
//! this is tested on any machine.

use std::path::{Path, PathBuf};

/// Where the per-user install lives: the exe and its Start Menu shortcut.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    /// The install folder: `%LocalAppData%\Programs\Island`.
    pub dir: PathBuf,
    /// The installed binary: `dir\island.exe`.
    pub exe: PathBuf,
    /// The Start Menu shortcut: `%AppData%\Microsoft\Windows\Start Menu\Programs\Island.lnk`.
    pub shortcut: PathBuf,
}

/// The paths a per-user install uses, from the folders it lands in.
pub fn layout(local_app_data: &Path, app_data: &Path) -> Layout {
    let dir = local_app_data.join("Programs").join("Island");
    let exe = dir.join("island.exe");
    let shortcut = app_data
        .join("Microsoft")
        .join("Windows")
        .join("Start Menu")
        .join("Programs")
        .join("Island.lnk");
    Layout { dir, exe, shortcut }
}

/// The registry key the uninstall entry lives under.
pub const UNINSTALL_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\Island";

/// A registry value of the uninstall entry, as the app's side would write it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegValue {
    Str(String),
    Dword(u32),
}

/// The uninstall entry's values for an installed exe: what Settings' Apps
/// page shows, and how the quiet uninstall runs. `EstimatedSize` is in KB,
/// rounded up (0 stays 0).
pub fn uninstall_values(
    exe: &Path,
    version: &str,
    size_bytes: u64,
) -> Vec<(&'static str, RegValue)> {
    let dir = exe.parent().unwrap_or(exe).to_string_lossy().into_owned();
    let size_kb = u32::try_from(size_bytes.div_ceil(1024)).unwrap_or(u32::MAX);
    vec![
        ("DisplayName", RegValue::Str("Island".into())),
        ("DisplayVersion", RegValue::Str(version.into())),
        ("Publisher", RegValue::Str("thealejandro".into())),
        ("DisplayIcon", RegValue::Str(format!("{},0", quoted(exe)))),
        ("InstallLocation", RegValue::Str(dir)),
        (
            "UninstallString",
            RegValue::Str(format!("{} --uninstall", quoted(exe))),
        ),
        (
            "QuietUninstallString",
            RegValue::Str(format!("{} --uninstall --quiet", quoted(exe))),
        ),
        ("NoModify", RegValue::Dword(1)),
        ("NoRepair", RegValue::Dword(1)),
        ("EstimatedSize", RegValue::Dword(size_kb)),
    ]
}

/// `"path"`: a path with quotes around it, as commands and icons use it.
pub fn quoted(exe: &Path) -> String {
    format!("\"{}\"", exe.display())
}

/// The same path twice: case aside (`Island.EXE`), `/` vs `\`, and a
/// trailing separator.
pub fn same_path(a: &Path, b: &Path) -> bool {
    fn norm(p: &Path) -> String {
        p.to_string_lossy()
            .replace('/', "\\")
            .trim_end_matches('\\')
            .to_ascii_lowercase()
    }
    norm(a) == norm(b)
}

/// Whether a Run value ("Start with Windows") points at an island.exe other
/// than the installed one: empty (off) and already pointing there are not;
/// quoted or not, with arguments after the closing quote, it is the path at
/// the start that counts.
pub fn run_points_elsewhere(run_value: &str, installed_exe: &Path) -> bool {
    let path = run_value.trim();
    let path = match path.strip_prefix('"') {
        Some(rest) => rest.split('"').next().unwrap_or(""),
        None => path.split_whitespace().next().unwrap_or(""),
    };
    !path.is_empty() && !same_path(Path::new(path), installed_exe)
}

/// What the island offers when it is opened from outside its install
/// folder (the exe just downloaded, say).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Offer {
    /// Nothing installed yet.
    Install,
    /// An older one is installed.
    Update,
}

/// The offer for this exe, if any: none for the installed one itself, for
/// one no newer than what is installed, or for the very version the user
/// chose to use without installing (a newer one asks again).
pub fn offer(
    is_installed: bool,
    installed: Option<&str>,
    this: &str,
    declined: Option<&str>,
) -> Option<Offer> {
    if is_installed || declined.is_some_and(|d| d.trim() == this) {
        return None;
    }
    match installed {
        None => Some(Offer::Install),
        Some(v) if newer(this, v) => Some(Offer::Update),
        Some(_) => None,
    }
}

/// What a newer exe tells the installed island that is running, so that one
/// offers the update: its version and where it is.
pub fn offer_text(version: &str, exe: &str) -> String {
    format!("{version}\n{exe}")
}

/// The version and the exe in an `offer_text`.
pub fn read_offer_text(s: &str) -> Option<(&str, &str)> {
    let (v, exe) = s.split_once('\n')?;
    (!v.trim().is_empty() && !exe.trim().is_empty()).then_some((v.trim(), exe.trim()))
}

/// Whether version `a` comes after `b` (`0.10.0` after `0.9.3`); parts that
/// aren't numbers count as 0.
pub fn newer(a: &str, b: &str) -> bool {
    let parts = |v: &str| -> Vec<u32> {
        v.trim()
            .split(['.', '-', '+'])
            .take(3)
            .map(|p| p.parse().unwrap_or(0))
            .collect()
    };
    let (mut a, mut b) = (parts(a), parts(b));
    a.resize(3, 0);
    b.resize(3, 0);
    a > b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_offers_to_install_or_update_only_when_it_helps() {
        assert_eq!(offer(false, None, "0.1.0", None), Some(Offer::Install));
        assert_eq!(
            offer(false, Some("0.1.0"), "0.2.0", None),
            Some(Offer::Update)
        );
        assert_eq!(offer(false, Some("0.2.0"), "0.2.0", None), None);
        assert_eq!(offer(false, Some("0.3.0"), "0.2.0", None), None);
        assert_eq!(offer(true, None, "0.1.0", None), None);
        // "Use without installing" holds for that version only.
        assert_eq!(offer(false, None, "0.1.0", Some("0.1.0\n")), None);
        assert_eq!(
            offer(false, None, "0.2.0", Some("0.1.0")),
            Some(Offer::Install)
        );
    }

    #[test]
    fn the_offer_text_goes_and_comes_back() {
        let t = offer_text("0.2.0", r"C:\Descargas\island.exe");
        assert_eq!(
            read_offer_text(&t),
            Some(("0.2.0", r"C:\Descargas\island.exe"))
        );
        assert_eq!(read_offer_text("0.2.0"), None);
        assert_eq!(read_offer_text("\nC:\\island.exe"), None);
        assert_eq!(read_offer_text("0.2.0\n "), None);
    }

    #[test]
    fn versions_compare_by_number() {
        assert!(newer("0.10.0", "0.9.3"));
        assert!(newer("1.0", "0.9.9"));
        assert!(!newer("0.1.0", "0.1.0"));
        assert!(!newer("0.1.0", "0.1.1"));
        assert!(newer("0.1.1", "junk"));
    }

    fn l() -> Layout {
        layout(
            Path::new(r"C:\Users\Test\AppData\Local"),
            Path::new(r"C:\Users\Test\AppData\Roaming"),
        )
    }

    #[test]
    fn the_layout_lands_in_the_users_folders() {
        let l = l();
        assert_eq!(
            l.dir,
            Path::new(r"C:\Users\Test\AppData\Local\Programs\Island")
        );
        assert_eq!(l.exe, l.dir.join("island.exe"));
        assert_eq!(
            l.shortcut,
            Path::new(
                r"C:\Users\Test\AppData\Roaming\Microsoft\Windows\Start Menu\Programs\Island.lnk"
            )
        );
    }

    #[test]
    fn the_uninstall_key_is_islands_own() {
        assert_eq!(
            UNINSTALL_KEY,
            "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\Island"
        );
    }

    fn value<'a>(values: &'a [(&'static str, RegValue)], key: &str) -> &'a RegValue {
        values
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| v)
            .unwrap_or_else(|| panic!("no {key}"))
    }

    #[test]
    fn the_uninstall_entry_describes_the_install() {
        let exe = Path::new(r"C:\Users\Test\AppData\Local\Programs\Island\island.exe");
        let values = uninstall_values(exe, "1.2.3", 3 * 1024);
        assert_eq!(
            value(&values, "DisplayName"),
            &RegValue::Str("Island".into())
        );
        assert_eq!(
            value(&values, "DisplayVersion"),
            &RegValue::Str("1.2.3".into())
        );
        assert_eq!(
            value(&values, "Publisher"),
            &RegValue::Str("thealejandro".into())
        );
        assert_eq!(
            value(&values, "DisplayIcon"),
            &RegValue::Str(r#""C:\Users\Test\AppData\Local\Programs\Island\island.exe",0"#.into())
        );
        assert_eq!(
            value(&values, "InstallLocation"),
            &RegValue::Str(r"C:\Users\Test\AppData\Local\Programs\Island".into())
        );
        assert_eq!(
            value(&values, "UninstallString"),
            &RegValue::Str(
                r#""C:\Users\Test\AppData\Local\Programs\Island\island.exe" --uninstall"#.into()
            )
        );
        assert_eq!(
            value(&values, "QuietUninstallString"),
            &RegValue::Str(
                r#""C:\Users\Test\AppData\Local\Programs\Island\island.exe" --uninstall --quiet"#
                    .into()
            )
        );
        assert_eq!(value(&values, "NoModify"), &RegValue::Dword(1));
        assert_eq!(value(&values, "NoRepair"), &RegValue::Dword(1));
        assert_eq!(value(&values, "EstimatedSize"), &RegValue::Dword(3));
    }

    #[test]
    fn the_estimated_size_rounds_up_to_whole_kb() {
        let exe = Path::new(r"C:\Island\island.exe");
        let kb = |size| match value(&uninstall_values(exe, "1", size), "EstimatedSize") {
            RegValue::Dword(kb) => *kb,
            _ => panic!("not a dword"),
        };
        assert_eq!(kb(0), 0);
        assert_eq!(kb(1), 1);
        assert_eq!(kb(1024), 1);
        assert_eq!(kb(1025), 2);
        assert_eq!(kb(2048), 2);
    }

    #[test]
    fn quoting_wraps_the_path_in_quotes() {
        assert_eq!(
            quoted(Path::new(r"C:\Island\island.exe")),
            r#""C:\Island\island.exe""#
        );
    }

    #[test]
    fn the_same_path_ignores_case_separators_and_a_trailing_one() {
        let base = Path::new(r"C:\Users\Test\AppData\Local\Programs\Island\island.exe");
        for same in [
            r"C:\Users\Test\AppData\Local\Programs\Island\ISLAND.EXE",
            r"c:\users\test\appdata\local\programs\island\island.exe",
            r"C:/Users/Test/AppData/Local/Programs/Island/island.exe",
            r"C:\Users\Test\AppData\Local\Programs\Island\island.exe\",
        ] {
            assert!(same_path(base, Path::new(same)), "{same}");
        }
        for other in [
            r"C:\Users\Test\AppData\Local\Programs\Island\other.exe",
            r"C:\Users\Test\AppData\Local\Programs\Island2\island.exe",
            r"C:\Users\Test\AppData\Local\Programs\Island\island.exe2",
        ] {
            assert!(!same_path(base, Path::new(other)), "{other}");
        }
    }

    #[test]
    fn the_run_value_is_elsewhere_only_when_it_points_at_another_island() {
        let exe = Path::new(r"C:\Users\Test\AppData\Local\Programs\Island\island.exe");
        // Off, or already pointing at the installed exe: nothing to repoint.
        assert!(!run_points_elsewhere("", exe));
        assert!(!run_points_elsewhere("   ", exe));
        assert!(!run_points_elsewhere(
            r#""C:\Users\Test\AppData\Local\Programs\Island\island.exe""#,
            exe
        ));
        assert!(!run_points_elsewhere(
            r"C:\Users\Test\AppData\Local\Programs\Island\island.exe",
            exe
        ));
        assert!(!run_points_elsewhere(
            r"C:\Users\Test\AppData\Local\Programs\Island\ISLAND.EXE",
            exe
        ));
        assert!(!run_points_elsewhere(
            r"C:/Users/Test/AppData/Local/Programs/Island/island.exe\",
            exe
        ));
        // Arguments after the closing quote are not part of the path.
        assert!(!run_points_elsewhere(
            r#""C:\Users\Test\AppData\Local\Programs\Island\island.exe" --minimized"#,
            exe
        ));
        // A different island.exe: repoint it.
        assert!(run_points_elsewhere(r"C:\Tools\island\island.exe", exe));
        assert!(run_points_elsewhere(
            r#""C:\Tools\island\island.exe" --quiet"#,
            exe
        ));
        assert!(run_points_elsewhere(
            r"C:\Users\Test\AppData\Local\Programs\Island\old\island.exe",
            exe
        ));
    }
}
