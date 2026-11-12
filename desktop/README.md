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

## Download

Each `windows-vX.Y.Z` tag publishes a GitHub Release (never marked Latest:
that one is the Omarchy plugin's) with `Island-X.Y.Z-x64.exe`, built by CI
with MSVC, and `SHA256SUMS.txt`. Its provenance can be checked with
`gh attestation verify Island-X.Y.Z-x64.exe --repo thealejandro/island`.

To publish one: set `version` in `desktop/Cargo.toml`, merge it into main, then tag
that commit `windows-vX.Y.Z` and push the tag. CI stops if the two differ,
or if the tagged commit is not on main.

## Install

Opened from anywhere else (just downloaded, say), the island offers it in
a large menu of its own, in the middle of the screen in use: "Install Island", or
"Update Island" when an older one is installed (shown by that one, if it is
running, which keeps running until the user chooses). If installing fails,
the island comes back and says so: after a failed update, the installed
one, whole, since the new exe is copied beside it first and only then put
in its place. "Use without installing" keeps it where
it is and isn't asked again for that version
(`%LOCALAPPDATA%\island\portable.txt`).

`island.exe --install` does the same by hand, for this user, with no admin
rights:

- it copies itself to `%LOCALAPPDATA%\Programs\Island`;
- it adds Island to the Start menu and to Settings > Apps > Installed apps;
- it starts the installed copy, which says so with a glance.

If "Start with Windows" was on, it now starts the installed copy. Opening
the island again while it runs opens its menu. Uninstalling it from
"Installed apps" (or `island.exe --uninstall`) first turns Windows'
banners back on, then removes the Run value, the shortcut, the entry and
the program's folder. The settings and the lyrics cache stay.

The island lives in the tray: its icon's menu, compact like Windows' own,
switches the lyrics and the lyrics column, starts it with Windows, opens
the settings and quits. The island itself keeps its clicks for the music
and the glances: a right click on it opens nothing. Starting with Windows is a value under the user's
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
| `monitor` | `primary`, a screen's number counted left to right, `all`, or a list of screens like `[1, 3]` (`1,3` from `--monitor`) | `primary` |
| `textAlign` | `center`, `left` | `center` |
| `songText` | `title-artist`, `artist-title`, `title`, `artist` | `title-artist` |
| `lyrics` | `true`, `false` | `false` |
| `openLyrics` | `true`, `false` | `true` |
| `openWidth` | 480 to 960, in steps of 10, or `auto` | `auto` |
| `islandScale` | 0.8 to 1.4 | `1.0` |
| `accent` | `system`, `white`, `theme`, or a color like `#ff8000` (the glances' levels and logo-less notifications; a color too dark to see on black is lightened to a 3:1 contrast) | `system` |
| `islandColor` | `black` (like a notch), `theme`, or a color like `#1e2a3a` (one too light for the white text is darkened to a 4.5:1 contrast) | `black` |
| `defaultPage` | `home`, `focus` | `home` |
| `focusMinutes` | 5 to 120 | `25` |
| `breakMinutes` | 1 to 30 | `5` |
| `longBreakMinutes` | 5 to 60 | `15` |
| `longBreakEvery` | 2 to 8 (rounds of focus) | `4` |
| `volumeGlance` | `true`, `false` | `true` |
| `hideWindowsFlyout` | `true`, `false` | `true` |
| `privacyGlance` | `true`, `false` | `true` |
| `bluetoothGlance` | `true`, `false` | `true` |
| `notificationGlance` | `true`, `false` | `true` |
| `notificationsInFullScreen` | `true`, `false` | `true` |
| `notificationSound` | `true`, `false` | `true` |
| `lyricsDelay` | `auto`, or 0 to 2000 (ms) | `auto` |

While music plays (any app in Windows' media controls), the closed island
shows the cover, the song and bars that move with the music; a new song
peeks for a few seconds, and the open island has the progress and the
controls, in the cover's color (the progress, the play button, the tab,
the halo and light behind the cover, the lyrics around the sung line
and the glow; white for a gray cover). When an unplugged screen was chosen, the island goes to the
primary until it is back. With more than one screen chosen (`all` or a
list), each one gets its own island: each opens and closes on its own, and
the glances (a new song, the volume, a notification) show on all of them
at once.

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

`islandScale` grows or shrinks the closed island and every glance (the
song, the volume, Bluetooth, the microphone and camera, notifications) by
a factor between 0.8 and 1.4, everything together; the open island keeps
its size. It is applied where the island sits and as it moves, so changing
it doesn't jolt the motion.

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

With it on, Windows' own volume flyout doesn't show over the glance: the
island takes the volume keys (up, down, mute) and moves the volume as they
do, 2 % a press, unmuting first. While an app is in Windows' media
controls, the play/pause, next and previous keys go to it the same way;
with none they go on to Windows. Windows 11 draws its flyout inside the
shell, with no window that could be hidden, so a key it never gets is the
one way. It's a keyboard hook on a thread of its own that waits between
presses, and every other key passes untouched. Keys meant for an app
running as administrator never reach it, so those keep Windows' flyout.
`hideWindowsFlyout: false` gives the keys back to Windows; so does
quitting the island.

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
shell's count of notifications changes, so nothing runs in between. A
notification with an image of its own (a toast's `appLogoOverride`, like a
web app's profile picture) shows that instead of the app's logo: the image
is read from the toast's XML in Windows' notification database, opened
read-only.

It doesn't need Windows' banner. "Notifications only in the island" in its
menu turns off "Show notification banners" for every app that has sent
notifications (as the switch under Settings > System > Notifications would;
Windows' own alerts keep theirs), and for each new one as its first
notification arrives; they still go to the notification center, and the
island still shows them. It applies at once and stays so, also after
restarting, until it is chosen again: then the banners it turned off (noted
in `%LOCALAPPDATA%\island\banners.txt`) come back, but not those turned off
by hand. Before uninstalling the island, choose it again, or run
`island.exe --restore-banners` (it does the same without opening the
island); else turn them on in Settings > System > Notifications.
With the banners off, the island plays Windows' own notification sound
when one arrives, since Windows itself no longer does
(`notificationSound: false` keeps it silent).
With an app in full screen the glance shows too
(`notificationsInFullScreen: false` keeps it away then); never while
presenting, in a game in exclusive full screen or with Do Not Disturb
(except the one Windows 11 turns on by itself in full screen).
A click on the glance opens the app that sent it (Windows lets no other app
click the notification itself). `notificationGlance: false` turns it off.

The icons are filled ones from Microsoft's
[Fluent UI System Icons](https://github.com/microsoft/fluentui-system-icons)
(MIT License, Copyright (c) 2020 Microsoft Corporation), drawn as shapes,
so they look the same on Windows 10 and 11 and at any scale.

For trying things out, the command line wins over the file:
`--placement`, `--monitor`, `--text-align`, `--song-text`, `--lyrics`,
`--open-lyrics on|off`, `--open-width` and `--page home|focus`.
CI builds with the MSVC toolchain. Without the Visual Studio Build Tools,
the `stable-x86_64-pc-windows-gnullvm` toolchain works too:
`cargo +stable-x86_64-pc-windows-gnullvm build`. That build is for trying
things out: only the MSVC one links with `/DEPENDENTLOADFLAG:0x800` (the
DLLs it starts with come from System32 alone), so what is published is
built by CI with MSVC.
