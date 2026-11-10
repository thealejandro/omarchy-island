# Island desktop

The island as native desktop apps, outside Omarchy. The repo root is the
Omarchy plugin; this folder is a separate Rust workspace.

- `crates/island-core`: platform-neutral logic (lyrics, texts, settings).
- `apps/windows`: the Windows app (Win32 + WinRT through windows-rs).
- `apps/macos`: later (#18).

Plan and progress: #64.

## Build

```sh
cd desktop
cargo test --workspace
cargo build --release -p island-windows   # target/release/island.exe
```

For now `island.exe --placement top|floating|top-left|top-right` picks where
it sits, `--monitor N` the screen (counted left to right; the primary by
default, and again when screen N is unplugged), and a right click on the island closes it (a tray icon comes later).

While music plays (any app in Windows' media controls), the closed island
shows the cover, the song and bars that move with the music; a new song
peeks for a few seconds, and the open island has the progress and the
controls. `--text-align center|left` and
`--song-text title-artist|artist-title|title|artist` style the song's line
(they move to the settings in phase 5).

CI builds with the MSVC toolchain. Without the Visual Studio Build Tools,
the `stable-x86_64-pc-windows-gnullvm` toolchain works too:
`cargo +stable-x86_64-pc-windows-gnullvm build`.
