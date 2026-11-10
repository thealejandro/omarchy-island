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
| `notificationGlance` | `true`, `false` | `true` |
| `notificationsInFullScreen` | `true`, `false` | `true` |
| `windowsBanners` | `true`, `false` | `true` |
| `lyricsDelay` | `auto`, or 0 to 2000 (ms) | `auto` |

While music plays (any app in Windows' media controls), the closed island
shows the cover, the song and bars that move with the music; a new song
peeks for a few seconds, and the open island has the progress and the
controls, in the cover's color (the progress, the play button, the tab,
the halo and light behind the cover, the lyrics around the sung line
and the glow; white for a gray cover). When an unplugged screen was chosen, the island goes to the
primary until it is back.

`lyrics` turns on synced lyrics from [lrclib.net](https://lrclib.net)
(off by default: it sends the song's name there). The closed island then
shows the line being sung, and the open one a column of lyrics beside the
player: the sung line bright, the ones around it fading, scrolling up as
the song goes (pauses show as `• • •`). `openLyrics: false` leaves the
open island as just the player, the lyrics only in the closed one. Answers
are cached in `%LOCALAPPDATA%\island\lyrics` (the last 400 songs).
Over Bluetooth the sound comes out a little after the position the player
tells, so the lines follow later: with `lyricsDelay: "auto"`, 200 ms when
the default speakers are Bluetooth ones and none when they are wired,
looked at again on each song and change of speakers. A number of ms is
used instead whatever the speakers; raise it if the lines still come
early, lower it if late.

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
glances at it for a moment: the speaker, the percentage and a bar in
the light shade of Windows' accent (as its own volume flyout), or
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

When a notification pops up, the island grows into a glance of its own,
taller than the song's so the two never pass for each other: the app's
logo, its name and "now", the title in bold and the next line dim. Two
lights in the logo's color (the system's accent when the logo has none)
run once around the island's edge, while a ring beats three times and then
glows softly until the glance ends, 5 seconds later. It reads them with Windows' own listener
(allowed under Privacy, "Notification access") and looks only when the
shell's count of notifications changes, so nothing runs in between.

It doesn't need Windows' banner: to see notifications only in the island,
turn off "Show notification banners" for those apps under Settings >
System > Notifications (they still go to the notification center, and the
island still shows them). With an app in full screen the glance shows too
(`notificationsInFullScreen: false` keeps it away then); never while
presenting, in a game in exclusive full screen or with Do Not Disturb
(except the one Windows 11 turns on by itself in full screen).
A click on the glance opens the app that sent it (Windows lets no other app
click the notification itself). `windowsBanners: false` lets the island
tell notifications alone: it turns off Windows' banner for each app (as
the switch in Settings would; Windows' own alerts keep theirs), notes the
ones it turned off in `%LOCALAPPDATA%islandanners.txt`, and turns
those back on when the island quits or the setting goes back to `true`.
`notificationGlance: false` turns it off.

The icons are filled ones from Microsoft's
[Fluent UI System Icons](https://github.com/microsoft/fluentui-system-icons)
(MIT License, Copyright (c) 2020 Microsoft Corporation), drawn as shapes,
so they look the same on Windows 10 and 11 and at any scale.

For trying things out, the command line wins over the file:
`--placement`, `--monitor`, `--text-align`, `--song-text`, `--lyrics`,
`--open-lyrics on|off`, `--open-width` and `--page home|focus`.
CI builds with the MSVC toolchain. Without the Visual Studio Build Tools,
the `stable-x86_64-pc-windows-gnullvm` toolchain works too:
`cargo +stable-x86_64-pc-windows-gnullvm build`.
