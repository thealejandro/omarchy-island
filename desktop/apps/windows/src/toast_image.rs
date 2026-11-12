//! A notification's own image. A toast may carry one
//! (`<image placement="appLogoOverride">`, a web app's profile picture,
//! say); the system's public listener gives only the text, but the toast's
//! XML is in Windows' notification database. This reads it there — the
//! database opened read-only, only the one about to show, nothing ever
//! written — and takes the image's file, for the window to draw instead of
//! the app's logo. Any failure falls back to the logo, silently.

use std::ffi::{CString, c_char, c_int, c_void};
use std::path::PathBuf;
use std::ptr;

const SQLITE_OK: c_int = 0;
const SQLITE_ROW: c_int = 100;
const SQLITE_OPEN_READONLY: c_int = 0x1;
const SQLITE_OPEN_URI: c_int = 0x40;

/// Nothing bigger is an icon.
const MAX: u64 = 4 << 20;

#[link(name = "winsqlite3", kind = "raw-dylib")]
unsafe extern "system" {
    fn sqlite3_open_v2(
        filename: *const c_char,
        db: *mut *mut c_void,
        flags: c_int,
        vfs: *const c_char,
    ) -> c_int;
    fn sqlite3_busy_timeout(db: *mut c_void, ms: c_int) -> c_int;
    fn sqlite3_prepare_v2(
        db: *mut c_void,
        sql: *const c_char,
        n_byte: c_int,
        stmt: *mut *mut c_void,
        tail: *mut *const c_char,
    ) -> c_int;
    fn sqlite3_bind_int64(stmt: *mut c_void, index: c_int, value: i64) -> c_int;
    fn sqlite3_step(stmt: *mut c_void) -> c_int;
    fn sqlite3_column_blob(stmt: *mut c_void, column: c_int) -> *const c_void;
    fn sqlite3_column_bytes(stmt: *mut c_void, column: c_int) -> c_int;
    fn sqlite3_finalize(stmt: *mut c_void) -> c_int;
    fn sqlite3_close_v2(db: *mut c_void) -> c_int;
}

/// Closes the database even on error (it is a handle even when opening
/// failed).
struct Db(*mut c_void);

impl Db {
    /// The notification database, read-only; `None` when it can't be opened.
    fn open() -> Option<Self> {
        let local = std::env::var_os("LOCALAPPDATA")?;
        let path = PathBuf::from(local).join("Microsoft/Windows/Notifications/wpndatabase.db");
        // The URI form: forward slashes and `?mode=ro` on top of the
        // read-only flag, so nothing can touch it.
        let uri = format!("file:{}?mode=ro", uri_path(&path.to_string_lossy()));
        let uri = CString::new(uri).ok()?;
        let mut db = ptr::null_mut();
        let r = unsafe {
            sqlite3_open_v2(
                uri.as_ptr(),
                &mut db,
                SQLITE_OPEN_READONLY | SQLITE_OPEN_URI,
                ptr::null(),
            )
        };
        if r != SQLITE_OK || db.is_null() {
            if !db.is_null() {
                unsafe {
                    sqlite3_close_v2(db);
                }
            }
            return None;
        }
        let db = Self(db);
        // The database is busy while the shell works on it: wait briefly.
        unsafe {
            sqlite3_busy_timeout(db.0, 200);
        }
        Some(db)
    }

    fn prepare(&self, sql: &std::ffi::CStr) -> Option<Stmt> {
        let mut stmt = ptr::null_mut();
        let r = unsafe { sqlite3_prepare_v2(self.0, sql.as_ptr(), -1, &mut stmt, ptr::null_mut()) };
        (r == SQLITE_OK && !stmt.is_null()).then(|| Stmt(stmt))
    }
}

impl Drop for Db {
    fn drop(&mut self) {
        unsafe {
            sqlite3_close_v2(self.0);
        }
    }
}

/// Finalized even when stepping fails.
struct Stmt(*mut c_void);

impl Stmt {
    /// The one row's first column, as bytes; `None` without a row or with an
    /// empty one.
    fn row(&self) -> Option<Vec<u8>> {
        (unsafe { sqlite3_step(self.0) } == SQLITE_ROW).then(|| {
            let n = unsafe { sqlite3_column_bytes(self.0, 0) };
            let p = unsafe { sqlite3_column_blob(self.0, 0) };
            if n <= 0 || p.is_null() {
                Vec::new()
            } else {
                unsafe { std::slice::from_raw_parts(p.cast::<u8>(), n as usize) }.to_vec()
            }
        })
    }
}

impl Drop for Stmt {
    fn drop(&mut self) {
        unsafe {
            sqlite3_finalize(self.0);
        }
    }
}

/// A path as an SQLite URI's: `/` for `\`, and anything that could end or
/// bend the URI (`?`, `#`, `%`, a space, any non-ASCII) as `%XX`.
fn uri_path(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for b in path.replace('\\', "/").bytes() {
        if b.is_ascii_alphanumeric() || b"/:-_.~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// The toast XML of the notification `id`, kept in Windows' database.
fn payload(id: u32) -> Option<String> {
    let db = Db::open()?;
    let sql = c"SELECT Payload FROM Notification WHERE Id = ?1";
    let stmt = db.prepare(sql)?;
    unsafe {
        sqlite3_bind_int64(stmt.0, 1, i64::from(id));
    }
    String::from_utf8(stmt.row()?).ok()
}

/// The notification's own image (PNG/JPEG bytes), `None` to use the app's
/// logo instead.
pub fn image(id: u32) -> Option<Vec<u8>> {
    use std::io::Read;
    let xml = payload(id)?;
    let path = local_path(&unescape(app_logo_override(&xml)?)?)?;
    if !on_fixed_drive(&path) {
        return None;
    }
    // At most MAX read, however the file grows meanwhile.
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .ok()?
        .take(MAX)
        .read_to_end(&mut bytes)
        .ok()?;
    (!bytes.is_empty() && (bytes.len() as u64) < MAX).then_some(bytes)
}

/// The path is on a disk of this computer: not a drive mapped to a share.
fn on_fixed_drive(path: &std::path::Path) -> bool {
    use windows::Win32::Storage::FileSystem::GetDriveTypeW;
    let Some(letter) = path.to_str().and_then(|p| p.chars().next()) else {
        return false;
    };
    let root = windows::core::HSTRING::from(format!("{letter}:\\"));
    // DRIVE_FIXED
    unsafe { GetDriveTypeW(&root) == 3 }
}

/// An attribute's value with XML's entities (`&amp;`, `&#39;`…) decoded;
/// `None` if one is malformed.
fn unescape(s: &str) -> Option<String> {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        let end = rest[at..].find(';')? + at;
        let name = &rest[at + 1..end];
        let c = match name {
            "amp" => '&',
            "lt" => '<',
            "gt" => '>',
            "quot" => '"',
            "apos" => '\'',
            _ => {
                let n = if let Some(h) = name.strip_prefix("#x").or(name.strip_prefix("#X")) {
                    u32::from_str_radix(h, 16).ok()?
                } else {
                    name.strip_prefix('#')?.parse().ok()?
                };
                char::from_u32(n)?
            }
        };
        out.push(c);
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    Some(out)
}

/// The `src` of the first `<image placement="appLogoOverride">` in the
/// toast XML, element by element, quotes respected; `None` when there is
/// none.
fn app_logo_override(xml: &str) -> Option<&str> {
    let mut rest = xml;
    while let Some(at) = rest.find("<image") {
        rest = &rest[at..];
        // Not `<imageSomething` element.
        let after = rest.get("<image".len()..)?;
        if !after.starts_with(|c: char| c.is_whitespace() || c == '/' || c == '>') {
            rest = after;
            continue;
        }
        let end = rest.find('>')?;
        let tag = &rest[..end];
        if attr(tag, "placement") == Some("appLogoOverride")
            && let Some(src) = attr(tag, "src")
        {
            return Some(src);
        }
        rest = &rest[end..];
    }
    None
}

/// The value of an attribute in a tag's text (`<image a="b" ...`); quotes
/// single or double.
fn attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let mut rest = tag;
    while let Some(at) = rest.find(name) {
        let before = rest.as_bytes().get(at.wrapping_sub(1)).copied();
        let after = &rest[at + name.len()..];
        // A whole attribute name, not the tail of a longer one.
        if before.is_some_and(|b| b == b'_' || b.is_ascii_alphanumeric()) {
            rest = &rest[at + 1..];
            continue;
        }
        let after = after.trim_start();
        let Some(eq) = after.strip_prefix('=') else {
            rest = &rest[at + 1..];
            continue;
        };
        let value = eq.trim_start();
        let quote = value.as_bytes().first().copied()?;
        if quote != b'"' && quote != b'\'' {
            rest = &rest[at + 1..];
            continue;
        }
        let inner = &value[1..];
        let end = inner.find(quote as char)?;
        return Some(&inner[..end]);
    }
    None
}

/// A local file path for an image `src`: an absolute one (`C:\...`, also
/// with forward slashes) or a `file:///...` one; anything remote or
/// packaged (`http(s)`, `ms-appx`, `ms-appdata`) is no picture of ours.
fn local_path(src: &str) -> Option<PathBuf> {
    let src = src.trim();
    let path = src.strip_prefix("file:///").unwrap_or(src);
    if src != path {
        let decoded = percent_decode(path)?;
        return windows_path(&decoded);
    }
    windows_path(path)
}

/// A real path on this computer's drives: nothing else (no relative paths,
/// no other schemes, and no network shares, which a toast could point at
/// to have the island connect to another machine).
fn windows_path(path: &str) -> Option<PathBuf> {
    let bytes = path.as_bytes();
    let drive = bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/');
    drive.then(|| PathBuf::from(path))
}

/// `%XX` sequences decoded (as UTF-8); `None` if one is invalid.
fn percent_decode(s: &str) -> Option<String> {
    let mut out = Vec::with_capacity(s.len());
    let mut rest = s.as_bytes();
    while let Some(at) = rest.iter().position(|&b| b == b'%') {
        out.extend_from_slice(&rest[..at]);
        let hex = std::str::from_utf8(rest.get(at + 1..at + 3)?).ok()?;
        out.push(u8::from_str_radix(hex, 16).ok()?);
        rest = &rest[at + 3..];
    }
    out.extend_from_slice(rest);
    String::from_utf8(out).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn logo_of(xml: &str) -> Option<String> {
        app_logo_override(xml).map(|s| s.to_string())
    }

    #[test]
    fn takes_the_app_logo_override() {
        let xml = "<toast><visual><binding template=\"ToastGeneric\">\
            <image placement=\"appLogoOverride\" src=\"C:\\a\\p.png\"/>\
            <text>hi</text></binding></visual></toast>";
        assert_eq!(logo_of(xml).as_deref(), Some("C:\\a\\p.png"));
    }

    #[test]
    fn none_without_app_logo_override() {
        let xml = "<toast><image src=\"C:\\a\\p.png\"/>\
            <image placement=\"hero\" src=\"C:\\a\\h.jpg\"/></toast>";
        assert_eq!(logo_of(xml), None);
        assert_eq!(logo_of("<toast><text>hi</text></toast>"), None);
        assert_eq!(logo_of(""), None);
    }

    #[test]
    fn first_of_several_wins() {
        let xml = "<toast>\
            <image placement=\"appLogoOverride\" src=\"C:\\a\\1.png\"/>\
            <image placement=\"appLogoOverride\" src=\"C:\\a\\2.png\"/></toast>";
        assert_eq!(logo_of(xml).as_deref(), Some("C:\\a\\1.png"));
    }

    #[test]
    fn attributes_in_any_order_and_single_quotes() {
        let xml = "<toast><image src='C:\\a\\p.png' placement='appLogoOverride'/></toast>";
        assert_eq!(logo_of(xml).as_deref(), Some("C:\\a\\p.png"));
    }

    #[test]
    fn src_on_a_later_image_is_found() {
        let xml = "<toast><image src=\"C:\\a\\big.png\"/>\
            <image placement=\"appLogoOverride\" src=\"C:\\a\\p.png\"/></toast>";
        assert_eq!(logo_of(xml).as_deref(), Some("C:\\a\\p.png"));
    }

    #[test]
    fn not_an_image_element() {
        let xml = "<toast><imagex placement=\"appLogoOverride\" src=\"C:\\a\\p.png\"/></toast>";
        assert_eq!(logo_of(xml), None);
    }

    #[test]
    fn entities_and_uri() {
        assert_eq!(
            unescape("C:\\A &amp; B\\it&apos;s&#33;&#x41;.png").as_deref(),
            Some("C:\\A & B\\it's!A.png")
        );
        assert_eq!(unescape("a &bogus; b"), None);
        assert_eq!(
            uri_path("C:\\Users\\Jo #1?%\\x.db"),
            "C:/Users/Jo%20%231%3F%25/x.db"
        );
        assert_eq!(uri_path("C:\\Users\\José"), "C:/Users/Jos%C3%A9");
    }

    #[test]
    fn local_paths_only() {
        assert_eq!(
            local_path("C:\\a\\p.png"),
            Some(PathBuf::from("C:\\a\\p.png"))
        );
        assert_eq!(local_path("C:/a/p.png"), Some(PathBuf::from("C:/a/p.png")));
        assert_eq!(
            local_path("file:///C:/a/p%20c.png"),
            Some(PathBuf::from("C:/a/p c.png"))
        );
        assert_eq!(local_path(r"\\srv\share\p.png"), None);
        assert_eq!(local_path("file:////srv/share/p.png"), None);
        assert_eq!(
            local_path("file:///C:/fotos/Jos%C3%A9.png"),
            Some(PathBuf::from("C:/fotos/José.png"))
        );
        assert_eq!(local_path("p.png"), None);
        assert_eq!(local_path("C:"), None);
        assert_eq!(local_path("http://x/p.png"), None);
        assert_eq!(local_path("https://x/p.png"), None);
        assert_eq!(local_path("ms-appx:///p.png"), None);
        assert_eq!(local_path("ms-appdata:///local/p.png"), None);
        assert_eq!(local_path("file:///http:/bad"), None);
    }
}
