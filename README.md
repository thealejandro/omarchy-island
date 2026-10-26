# omarchy-island

[Español](README.es.md)

A dynamic island for [Omarchy](https://omarchy.org) that grows out of the
center of the Omarchy bar. It shows what's playing, gives you media controls,
the month calendar, a focus timer, the weather, quick actions and Omarchy's
quick toggles, and gets out of the way when you're done.

It is a regular Omarchy shell plugin: it runs inside `omarchy-shell`, takes
its colors, font and corner radius from your current theme, and doesn't
replace or modify anything in Omarchy. Remove it and your desktop is exactly
as it was.

## Features

- **Peek**: when the song changes or the timer ends, a one-line glance drops
  out of the bar for a few seconds.
- **Open**: hover the bar's clock, or just below it (or bind a key), to open
  the island: cover art, source player,
  title, artist, a progress bar you can click or drag, previous / play /
  next.
- **Calendar**: the current month with today highlighted. Arrows or the
  scroll wheel change month; click the title to go back to today.
- **Focus timer**: Pomodoro (25 / 5 by default) with a notification when each
  phase ends. Scroll over the timer to change the focus length (5–120 min).
- **Dock**: one slim row along the bottom.
  - Quick toggles: night light, silence notifications, stay awake, screen
    recording and reminders, the same switches as Omarchy's bar indicators.
    A lit button is on.
  - Actions: screenshot, color picker, clipboard history and emoji picker.
    The island steps aside before they start.
  - Weather: icon, temperature and conditions for the place set in Omarchy's
    weather widget (or by IP), from wttr.in. Click it for Omarchy's forecast.
  - Hover a button to see its name.
- **No duplicate toggles**: with the island on the bar (see below), hovering
  the bar's center no longer reveals Omarchy's inactive indicators; the bar
  keeps only the ones that are on. Configurable.
- **English and Spanish**: follows `LANG`, or set it in `shell.json`.
- **Theme-aware**: follows `omarchy theme set` live: colors, font and accent.
  The island keeps rounded corners with concave "ears" that melt into the
  bar.
- **Light**: no extra process. Media comes from MPRIS (Spotify, browsers,
  mpv…); the only polling is the weather, every 20 minutes.

## Requirements

Omarchy 4 or newer (Quickshell-based `omarchy-shell`) with the bar at the top.

## Install

```bash
omarchy plugin add https://github.com/thealejandro/omarchy-island.git --enable
```

Then put it on the bar's center, next to the clock. It takes no room there;
it's what lets it hide the indicators the island already carries:

```bash
omarchy bar put thealejandro.island --after omarchy.clock
```

Remove it any time:

```bash
omarchy plugin remove thealejandro.island
```

## Usage

- Hover the bar's clock, or the thin strip right under it, to open the
  island; move away and it closes.
- Click the clock to pin the island open (its calendar replaces Omarchy's
  popup); click again to close it. Right click still changes the clock
  format and middle click still opens the timezone menu.
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

Optional, in the island's entry in `~/.config/omarchy/shell.json` (in the
bar layout, or under `plugins` if you didn't put it on the bar):

```json
{ "id": "thealejandro.island", "language": "es", "toggles": "island" }
```

Or from the command line: `omarchy bar set thealejandro.island toggles both`.

| Key        | Values                       | Default     |
|------------|------------------------------|-------------|
| `language` | `en`, `es`                   | from `LANG` |
| `clock`    | `true`, `false`              | `true`      |
| `toggles`  | `island`, `both`, `bar`      | `island`    |
| `units`    | `metric`, `imperial`         | `metric`    |

`toggles` decides where Omarchy's quick toggles live:

- `island`: in the island; the bar shows only the indicators that are on.
- `both`: in the island, and the bar still reveals all of them when you hover
  its center (Omarchy's own behavior).
- `bar`: only on the bar; the island's dock keeps the actions and weather.

With `"clock": false` the island leaves the clock alone: hover only the top
edge to open it, and a click on the clock opens Omarchy's calendar again.

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
| `Service.qml`      | State: MPRIS player, timer, toggles, weather, settings, IPC |
| `BarWidget.qml`    | Invisible bar entry that hides duplicate indicators  |
| `IslandModel.js`   | Pure helpers: player choice, clock, strings, month grid, weather |
| `IslandWindow.qml` | Per-screen layer surface, input mask, hover, motion  |
| `IslandShape.qml`  | The silhouette with concave ears                     |
| `PeekView.qml`     | The one-line glance                                  |
| `OpenView.qml`     | Layout of the open island                            |
| `MediaCard.qml`    | Cover, track, seek bar and transport                 |
| `CalendarView.qml` | Month calendar                                       |
| `TimerRow.qml`     | Focus timer                                          |
| `Dock.qml`         | Toggles, actions and weather row                     |
| `IslandButton.qml` | Round icon button                                    |
| `Glyph.qml`        | Icon centered by its ink, not its font box           |
| `Equalizer.qml`    | Small animated bars while music plays                |

Ideas and bugs go to the issues. Inspired by
[Nusa](https://rama2555.github.io/nusa/) on Windows and the notch in
[Ambxst](https://github.com/Axenide/Ambxst); no code is taken from either.

## License

MIT
