# omarchy-island

[Español](README.es.md)

A dynamic island for [Omarchy](https://omarchy.org) that grows out of the
center of the Omarchy bar. It shows what's playing, gives you media controls,
the month calendar and a focus timer, and gets out of the way when you're done.

It is a regular Omarchy shell plugin: it runs inside `omarchy-shell`, takes
its colors, font and corner radius from your current theme, and doesn't
replace or modify anything in Omarchy. Remove it and your desktop is exactly
as it was.

## Features

- **Peek**: when the song changes or the timer ends, a one-line glance drops
  out of the bar for a few seconds.
- **Open**: hover the top edge over the clock, or just below the center of
  the bar (or bind a key), to open the island: cover art, source player,
  title, artist, a progress bar you can click or drag, previous / play /
  next.
- **Calendar**: the current month with today highlighted. Arrows or the
  scroll wheel change month; click the title to go back to today.
- **Focus timer**: Pomodoro (25 / 5 by default) with a notification when each
  phase ends. Scroll over the time to change the focus length (5–120 min).
- **English and Spanish**: follows `LANG`, or set it in `shell.json`.
- **Theme-aware**: follows `omarchy theme set` live: colors, font and accent.
  The island keeps rounded corners with concave "ears" that melt into the
  bar.
- **Light**: no extra process, no polling while idle. Media comes from MPRIS
  (Spotify, browsers, mpv…).

## Requirements

Omarchy 4 or newer (Quickshell-based `omarchy-shell`) with the bar at the top.

## Install

```bash
omarchy plugin add https://github.com/thealejandro/omarchy-island.git --enable
```

Remove it any time:

```bash
omarchy plugin remove thealejandro.island
```

## Usage

- Hover the top edge of the screen over the clock, or the thin strip right
  under the bar's center, to open it; move away and it closes.
- Toggle it from a key, in `~/.config/hypr/bindings.lua`:

  ```lua
  o.bind("SUPER + I", "Island", "omarchy-shell island toggle")
  ```

IPC (`omarchy-shell island <method>`):

| Method   | Effect                                   |
|----------|------------------------------------------|
| `toggle` | Pin the island open / closed             |
| `open`   | Pin it open                              |
| `close`  | Close it                                 |
| `peek`   | Show the short glance                    |
| `timer`  | Start / pause the focus timer            |
| `focus <min>` | Set the focus length (idle timer, 5–120) |
| `state`  | JSON with the current state              |

## Settings

Optional, in the plugin's entry in `~/.config/omarchy/shell.json`:

```json
{ "id": "thealejandro.island", "language": "es" }
```

| Key        | Values       | Default            |
|------------|--------------|--------------------|
| `language` | `en`, `es`   | from `LANG`        |

## Development

Clone anywhere and link it into the plugin directory:

```bash
git clone https://github.com/thealejandro/omarchy-island.git
ln -s "$PWD/omarchy-island" ~/.config/omarchy/plugins/thealejandro.island
omarchy-shell shell rescanPlugins
omarchy plugin enable thealejandro.island
```

Quickshell caches compiled QML, so after editing the code run
`omarchy restart shell` to see the changes.

| File               | What it does                                         |
|--------------------|------------------------------------------------------|
| `Service.qml`      | State: MPRIS player, focus timer, peek, settings, IPC |
| `IslandModel.js`   | Pure helpers: player choice, clock, strings, month grid |
| `IslandWindow.qml` | Per-screen layer surface, input mask, hover, motion  |
| `IslandShape.qml`  | The silhouette with concave ears                     |
| `PeekView.qml`     | The one-line glance                                  |
| `OpenView.qml`     | Layout of the open island                            |
| `MediaCard.qml`    | Cover, track, seek bar and transport                 |
| `CalendarView.qml` | Month calendar                                       |
| `TimerRow.qml`     | Focus timer                                          |
| `IslandButton.qml` | Round icon button                                    |
| `Equalizer.qml`    | Small animated bars while music plays                |

Ideas and bugs go to the issues. Inspired by
[Nusa](https://rama2555.github.io/nusa/) on Windows and the notch in
[Ambxst](https://github.com/Axenide/Ambxst); no code is taken from either.

## License

MIT
