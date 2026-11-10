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

The island lives in the tray: its menu (also a right click on the island)
switches the lyrics and the lyrics column, starts it with Windows, opens
the settings and quits. Starting with Windows is a value under the user's
own `Run` key, written only when chosen there.

## Settings

`%APPDATA%\island\settings.json`, made with every key at its default the
first time "Open settings" is chosen. Changes apply as soon as the file is
saved; a value that can't be used falls back to its default, and a file
that isn't valid JSON is ignored until it is fixed.

| Key | Values | Default |
| --- | --- | --- |
| `language` | `""` (the system's), `en`, `es` | `""` |
| `placement` | `top`, `floating`, `top-left`, `top-right` | `top` |
| `monitor` | `primary`, or a screen's number counted left to right | `primary` |
| `textAlign` | `center`, `left` | `center` |
| `songText` | `title-artist`, `artist-title`, `title`, `artist` | `title-artist` |
| `lyrics` | `true`, `false` | `false` |
| `openLyrics` | `true`, `false` | `true` |
| `openWidth` | 480 to 960, in steps of 10, or `auto` | `auto` |

While music plays (any app in Windows' media controls), the closed island
shows the cover, the song and bars that move with the music; a new song
peeks for a few seconds, and the open island has the progress and the
controls. When an unplugged screen was chosen, the island goes to the
primary until it is back.

`lyrics` turns on synced lyrics from [lrclib.net](https://lrclib.net)
(off by default: it sends the song's name there). The closed island then
shows the line being sung, and the open one a column of lyrics beside the
player: the sung line bright, the ones around it fading, scrolling up as
the song goes (pauses show as `• • •`). `openLyrics: false` leaves the
open island as just the player, the lyrics only in the closed one. Answers
are cached in `%LOCALAPPDATA%\island\lyrics` (the last 400 songs).

`openWidth` is the open island's width (`auto`: 560, with the lyrics
column or without). The column takes a share of it, or on a narrow island
what the player can spare; below 500 there is no room for one and only the
player shows. On small or scaled-up
screens the island takes at most 60 % of the screen's width (610 on a
1024-wide one, 760 on a 1280 x 1024 square monitor), down to 480.

For trying things out, the command line wins over the file:
`--placement`, `--monitor`, `--text-align`, `--song-text`, `--lyrics`,
`--open-lyrics on|off` and `--open-width`.
CI builds with the MSVC toolchain. Without the Visual Studio Build Tools,
the `stable-x86_64-pc-windows-gnullvm` toolchain works too:
`cargo +stable-x86_64-pc-windows-gnullvm build`.
