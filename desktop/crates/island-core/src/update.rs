//! Checking for a newer island on GitHub Releases, when the user asks: which
//! release (from the API's list), where its files are, and the checksum to
//! hold the download to. Everything from the network is untrusted.
//!
//! The repo's releases are shared with the Omarchy plugin: only tags
//! `windows-vX.Y.Z` (nothing after the numbers) are the desktop's.

use serde_json::Value;

pub const HOST: &str = "github.com";
pub const API_HOST: &str = "api.github.com";
/// The newest 100 releases, the plugin's among them: an island older than
/// that many releases is not told about the newest.
pub const RELEASES_PATH: &str = "/repos/thealejandro/island/releases?per_page=100";
/// The releases page, for a copy that can't update itself.
pub const RELEASES_PAGE: &str = "https://github.com/thealejandro/island/releases";
pub const TAG_PREFIX: &str = "windows-v";
pub const SUMS: &str = "SHA256SUMS.txt";
/// Bigger than any exe the island makes (a few MB): a release's file over
/// it is not one.
pub const MAX_EXE: u64 = 64 << 20;
/// The API's answer for 100 releases, and the checksums file.
pub const MAX_LIST: usize = 4 << 20;
pub const MAX_SUMS: usize = 64 << 10;

/// `a.b.c`, each part as Windows keeps it in an exe's version (16 bits).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version(pub u16, pub u16, pub u16);

impl Version {
    /// Only three numbers and two dots: `0.2.0-rc.1`, `v1.0` or `1.0.0.0`
    /// are not versions here.
    pub fn parse(s: &str) -> Option<Self> {
        let mut parts = s.split('.').map(|p| {
            (!p.is_empty() && p.len() <= 5 && p.bytes().all(|b| b.is_ascii_digit()))
                .then(|| p.parse::<u16>().ok())
                .flatten()
        });
        let v = Self(parts.next()??, parts.next()??, parts.next()??);
        parts.next().is_none().then_some(v)
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.0, self.1, self.2)
    }
}

/// The version a desktop tag is of; `None` for any other tag.
pub fn tag_version(tag: &str) -> Option<Version> {
    Version::parse(tag.strip_prefix(TAG_PREFIX)?)
}

/// The exe CI publishes for `v` (x64, the only one built).
pub fn asset_name(v: Version) -> String {
    format!("Island-{v}-x64.exe")
}

/// Where a release's file is, on [`HOST`]: built from the tag, never taken
/// from the API's answer.
pub fn download_path(tag: &str, name: &str) -> String {
    format!("/thealejandro/island/releases/download/{tag}/{name}")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    pub tag: String,
    pub version: Version,
}

/// The newest release above `running` with both its exe and its checksums:
/// published (no drafts or prereleases), a desktop tag, and files of a
/// size that makes sense. `None` when there is none.
pub fn pick(list: &Value, running: Version) -> Option<Release> {
    list.as_array()?
        .iter()
        .filter_map(|r| {
            if r["draft"].as_bool() != Some(false) || r["prerelease"].as_bool() != Some(false) {
                return None;
            }
            let tag = r["tag_name"].as_str()?;
            let version = tag_version(tag)?;
            if version <= running {
                return None;
            }
            let assets = r["assets"].as_array()?;
            let has = |name: &str, max: u64| {
                assets.iter().any(|a| {
                    a["name"].as_str() == Some(name)
                        && a["size"].as_u64().is_some_and(|s| s > 0 && s <= max)
                })
            };
            (has(&asset_name(version), MAX_EXE) && has(SUMS, MAX_SUMS as u64)).then(|| Release {
                tag: tag.to_owned(),
                version,
            })
        })
        .max_by_key(|r| r.version)
}

/// The SHA-256 `SHA256SUMS.txt` gives for `name` (lines `<hex>  <name>`, as
/// `sha256sum` writes them, `*` before binary names; CRLF or LF).
pub fn sums_hash(text: &str, name: &str) -> Option<[u8; 32]> {
    text.lines().find_map(|line| {
        let (hex, rest) = line.trim_end_matches('\r').split_once(' ')?;
        let file = rest.trim_start_matches(' ');
        (file.strip_prefix('*').unwrap_or(file) == name).then(|| from_hex(hex))?
    })
}

fn from_hex(hex: &str) -> Option<[u8; 32]> {
    if hex.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, b) in out.iter_mut().enumerate() {
        *b = u8::from_str_radix(hex.get(2 * i..2 * i + 2)?, 16).ok()?;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn versions_are_three_numbers() {
        assert_eq!(Version::parse("0.1.5"), Some(Version(0, 1, 5)));
        assert_eq!(Version::parse("10.20.65535"), Some(Version(10, 20, 65535)));
        for bad in [
            "",
            "1",
            "1.2",
            "1.2.3.4",
            "1.2.3-rc.1",
            "1.2.3+b",
            "v1.2.3",
            "1..3",
            "1.2.65536",
            "1.2.+3",
            " 1.2.3",
            "1.2.0003x",
        ] {
            assert_eq!(Version::parse(bad), None, "{bad}");
        }
        assert!(Version(0, 10, 0) > Version(0, 9, 9));
        assert_eq!(Version(1, 2, 3).to_string(), "1.2.3");
    }

    #[test]
    fn only_desktop_tags() {
        assert_eq!(tag_version("windows-v0.2.0"), Some(Version(0, 2, 0)));
        for other in [
            "v0.2.0",
            "0.2.0",
            "windows-v0.2.0-rc.1",
            "windows-0.2.0",
            "macos-v0.2.0",
        ] {
            assert_eq!(tag_version(other), None, "{other}");
        }
    }

    #[test]
    fn files_come_from_the_tag() {
        assert_eq!(asset_name(Version(0, 2, 0)), "Island-0.2.0-x64.exe");
        assert_eq!(
            download_path("windows-v0.2.0", SUMS),
            "/thealejandro/island/releases/download/windows-v0.2.0/SHA256SUMS.txt"
        );
    }

    fn release(tag: &str, files: &[(&str, u64)]) -> Value {
        json!({
            "tag_name": tag,
            "draft": false,
            "prerelease": false,
            "assets": files.iter().map(|(n, s)| json!({"name": n, "size": s})).collect::<Vec<_>>(),
        })
    }

    fn full(v: &str) -> Value {
        release(
            &format!("windows-v{v}"),
            &[(&format!("Island-{v}-x64.exe"), 3_000_000), (SUMS, 90)],
        )
    }

    #[test]
    fn picks_the_newest_complete_one_above_the_running() {
        let running = Version(0, 1, 5);
        let mut draft = full("0.4.0");
        draft["draft"] = json!(true);
        let mut pre = full("0.3.5");
        pre["prerelease"] = json!(true);
        let list = json!([
            release("v2.0.0", &[("island.tar.gz", 1000)]),
            draft,
            pre,
            // The newest has no exe (not uploaded yet): the one before it.
            release("windows-v0.3.0", &[(SUMS, 90)]),
            full("0.2.1"),
            full("0.2.0"),
            full("0.1.5"),
            full("0.1.4"),
        ]);
        let got = pick(&list, running).unwrap();
        assert_eq!(got.tag, "windows-v0.2.1");
        assert_eq!(got.version, Version(0, 2, 1));
        // The running one, or older: nothing.
        assert_eq!(pick(&json!([full("0.1.5"), full("0.1.4")]), running), None);
        assert_eq!(pick(&json!([]), running), None);
        assert_eq!(pick(&json!({"message": "rate limited"}), running), None);
    }

    #[test]
    fn files_of_no_sense_are_not_a_release() {
        let running = Version(0, 1, 0);
        let empty = release("windows-v0.2.0", &[("Island-0.2.0-x64.exe", 0), (SUMS, 90)]);
        let huge = release(
            "windows-v0.2.0",
            &[("Island-0.2.0-x64.exe", MAX_EXE + 1), (SUMS, 90)],
        );
        let other = release(
            "windows-v0.2.0",
            &[("Island-0.3.0-x64.exe", 3_000_000), (SUMS, 90)],
        );
        let mut unsure = full("0.2.0");
        unsure.as_object_mut().unwrap().remove("draft");
        for r in [empty, huge, other, unsure] {
            assert_eq!(pick(&json!([r]), running), None);
        }
    }

    #[test]
    fn reads_the_checksum_ci_writes() {
        let hash = "a3".repeat(32);
        let name = "Island-0.2.0-x64.exe";
        let want = [0xa3; 32];
        // PowerShell's Out-File: CRLF.
        assert_eq!(sums_hash(&format!("{hash}  {name}\r\n"), name), Some(want));
        assert_eq!(sums_hash(&format!("{hash} *{name}\n"), name), Some(want));
        let two = format!("{}  other.exe\n{hash}  {name}\n", "00".repeat(32));
        assert_eq!(sums_hash(&two, name), Some(want));
        // Only that exact name, and a whole hash.
        assert_eq!(sums_hash(&format!("{hash}  {name}.old"), name), None);
        assert_eq!(sums_hash(&format!("{hash}  x{name}"), name), None);
        assert_eq!(sums_hash(&format!("{}  {name}", &hash[2..]), name), None);
        assert_eq!(sums_hash(&format!("{}zz  {name}", &hash[2..]), name), None);
        assert_eq!(sums_hash("", name), None);
    }
}
