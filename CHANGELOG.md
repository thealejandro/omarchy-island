# Changelog

All notable changes to this project are documented here. The format is
based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the
project follows [Semantic Versioning](https://semver.org/).

## [0.5.0] - 2026-11-08

### Added

- The island works with the bar on any edge (top, bottom, left, right); its
  shape and ears turn to face the bar.
- It hangs from Omarchy's real clock wherever the clock is on the bar, and
  stays on screen near a corner. `anchor` (`clock`, `widget`, `center`) and
  `clockClick` (`pin`, `calendar`, `omarchy`) choose how it's placed and
  what a click on the clock does.
- Pages: Home, Calendar, World and Settings, with tabs in the open island,
  `defaultPage` and IPC `page <name>`.
- Calendar page with a local agenda in
  `~/.local/state/omarchy/island/events.json`: events with repeats, dots on
  days with events, the island's own reminder (peek and notification,
  `reminderLead`), one-click hand-off to `omarchy reminder`, Omarchy's
  active reminders, and `agendaCommand` for events from another calendar.
  IPC `event` and `events`.
- World clock from the cities of Omarchy's elsewhen widget (or the island's
  own `cities`), as chips on Home and a full World page; `hour24`.
- 3-day forecast on the World page from the weather report already fetched,
  with links to Omarchy's forecast and location picker; `forecast`.
- A visual settings page for every setting, saved to `shell.json` right
  away and kept in sync with `omarchy bar set`; Reset; buttons to take over
  or give back Omarchy's OSD and to place the island next to the clock.
- More settings: `openWidth`, `blur`, `hoverOpen`, `hoverDelay`,
  `peekDuration`, `clockWidth`, `calendarFirst`, `showMedia`, `showTimer`,
  `showDock`, `showWorld`, `weekStart`, `widgetIcon` (an optional button of
  its own on the bar).
- `barWidget.defaults` and `barWidget.schema` in `manifest.json`.
- IPC `anchor` and `anchors`, for bars that can't report the clock to the
  island themselves.

### Changed

- `clock: false` now only turns off opening by hovering the clock.
- Text fields take the keyboard only while they're being edited.
- The settings page notes when the Omarchy version hasn't been tested.

### Fixed

- Settings were not read when the island sat in `centerBegin`, `centerEnd`
  or `tray`.
- Elsewhen and the weather on the bar could not be clicked while the island
  was open; now only the clock and the island itself take the pointer.
- The bar size is read per screen, so vertical bars place the island
  correctly.

## [0.4.0]

### Added

- Omarchy's OSD (volume, brightness…) in the island, battery peeks, theme
  border options for light themes.

[0.5.0]: https://github.com/thealejandro/omarchy-island/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/thealejandro/omarchy-island/releases/tag/v0.4.0
