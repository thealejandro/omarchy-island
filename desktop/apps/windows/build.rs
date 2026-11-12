//! The exe's icon (the tray's pill) and version, as Windows resources: what
//! Explorer, the Start menu and "Installed apps" show.

use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    // The DLLs the exe needs come from System32 only, never from beside it
    // (a Downloads or %TEMP% folder may hold others of the same name).
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        println!("cargo:rustc-link-arg-bins=/DEPENDENTLOADFLAG:0x800");
    }
    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR"));
    let ico = out.join("island.ico");
    std::fs::write(&ico, island_core::icon::ico(&[16, 20, 24, 32, 40, 48, 64]))
        .expect("island.ico");
    let version = env!("CARGO_PKG_VERSION");
    let mut n = version
        .split(['.', '-'])
        .map(|p| p.parse::<u16>().unwrap_or(0));
    let (a, b, c) = (
        n.next().unwrap_or(0),
        n.next().unwrap_or(0),
        n.next().unwrap_or(0),
    );
    let rc = format!(
        r#"1 ICON "{ico}"
1 VERSIONINFO
FILEVERSION {a},{b},{c},0
PRODUCTVERSION {a},{b},{c},0
FILEOS 0x40004
FILETYPE 1
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "040904B0"
    BEGIN
      VALUE "CompanyName", "thealejandro"
      VALUE "FileDescription", "Island"
      VALUE "FileVersion", "{version}"
      VALUE "InternalName", "island"
      VALUE "LegalCopyright", "MIT License"
      VALUE "OriginalFilename", "island.exe"
      VALUE "ProductName", "Island"
      VALUE "ProductVersion", "{version}"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x409, 1200
  END
END
"#,
        ico = ico.display().to_string().replace('\\', "\\\\"),
    );
    let rc_path = out.join("island.rc");
    std::fs::write(&rc_path, rc).expect("island.rc");
    embed_resource::compile(&rc_path, embed_resource::NONE)
        .manifest_optional()
        .expect("the exe's resources");
}
