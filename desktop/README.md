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
| `defaultPage` | `home`, `focus` | `home` |
| `focusMinutes` | 5 to 120 | `25` |
| `breakMinutes` | 1 to 30 | `5` |
| `longBreakMinutes` | 5 to 60 | `15` |
| `longBreakEvery` | 2 to 8 (rounds of focus) | `4` |
| `volumeGlance` | `true`, `false` | `true` |
| `privacyGlance` | `true`, `false` | `true` |
| `bluetoothGlance` | `true`, `false` | `true` |
| `lyricsDelay` | 0 to 2000 (ms) | `200` |

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
The sound comes out a little after the position the player tells (more
over Bluetooth), so the lines follow `lyricsDelay` ms later; raise it if
they still come early, lower it if late.

`openWidth` is the open island's width (`auto`: 560, with the lyrics
column or without). The column takes a share of it, or on a narrow island
what the player can spare; below 500 there is no room for one and only the
player shows. On small or scaled-up
screens the island takes at most 60 % of the screen's width (610 on a
1024-wide one, 760 on a 1280 x 1024 square monitor), down to 480.

The open island has tabs on top, with the time at the right: Home (the
player) and Focus, a Pomodoro timer. A break starts by itself when focus
ends, a long one every `longBreakEvery` rounds; the next focus waits for a
click. The glance tells when each ends, and with nothing playing the closed
island shows the time left. Before starting, the wheel over the time
changes `focusMinutes` by 5. The island opens on `defaultPage`, then on the
page last chosen.

When the volume changes (keys, taskbar or any app), the closed island
glances at it for a moment: the speaker, the percentage and a bar, or
"Muted". It follows the default speakers when they change, and nothing
runs while the volume stays put. `volumeGlance: false` turns it off.

When an app starts using the microphone or the camera, the island glances
at it with the app's name (orange for the microphone, green for the
camera), and again when the last one stops. It reads what Windows' privacy
settings keep and waits on the registry's own notice of a change.
`privacyGlance: false` turns it off.

A paired Bluetooth device that connects or disconnects (headphones, a
speaker, a keyboard) gets a glance with its name, once the change has held
for 5 seconds (some devices, phones above all, connect for a moment on
their own). Only paired devices are watched, so it never scans for new
ones. `bluetoothGlance: false` turns it
off.

For trying things out, the command line wins over the file:
`--placement`, `--monitor`, `--text-align`, `--song-text`, `--lyrics`,
`--open-lyrics on|off`, `--open-width` and `--page home|focus`.
CI builds with the MSVC toolchain. Without the Visual Studio Build Tools,
the `stable-x86_64-pc-windows-gnullvm` toolchain works too:
`cargo +stable-x86_64-pc-windows-gnullvm build`.
