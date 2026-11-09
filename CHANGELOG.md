# Changelog

All notable changes to this project are documented here. The format is
based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the
project follows [Semantic Versioning](https://semver.org/).

## [0.6.1] - 2026-11-08

### Fixed

- Disabling or removing the island (`omarchy plugin disable/remove`, a bar
  reset) gives Omarchy back its notifications and OSD if the island had
  them, instead of leaving the desktop without either. Omarchy has no
  uninstall hook, so the island checks a moment after it's unloaded: if its
  entry left `shell.json` it re-enables `omarchy.notifications` and
  `omarchy.osd`; on a shell restart the entry is still there and nothing
  changes (#30).

## [0.6.0] - 2026-11-08

### Added

- Notifications in the island (`notifications: island`, off by default):
  a glance for each new one, a Notifications page grouped by app with
  actions, inline replies, safe links and close per notification or app,
  do-not-disturb shared with Omarchy, and a 24-hour history in
  `~/.local/state/omarchy/island/notifications.json` (`notifyHistory`,
  `notifyHistoryLimit`). An unread dot under the clock and on the tab
  (`notifyBadge`), `notifyBody` and `notifyPeekDuration`.
- Taking over needs both the island's setting and Omarchy's
  `omarchy.notifications` disabled, so nothing ever shows twice; when
  Omarchy's plugin comes back by any route the island steps aside live,
  and if the island is removed or reset while it has them, Omarchy gets
  them back at the next start. Switch on the System section and IPC
  `island notifications <island|off>`. Omarchy's `notifications` IPC
  (`dismissAll`, `toggleDnd`, `invokeLast`, `probe`…) is answered by the
  island while it has them.
- A click on Omarchy's world clock or weather on the bar opens the island
  on the World page (`barClicks`, on by default); middle click still
  opens their own popup and right click does what the widget does.
- Light motion: the page change slides in from the side of its tab, the
  glance drops in, and `animations: false` turns every animation off.
- "Next" card on Home: the next three events of the agenda, a click opens
  that day on the Calendar page, and a button to add one.
- The latest two notifications on Home when the island has them.
- The author's logo in the READMEs and a credit line in Settings.

### Changed

- Home no longer shows the world clocks (they stay on the World page) and
  pairs what's playing with the "Next" card instead of the month.
  `showWorld` is gone; `calendarFirst` now puts "Next" on the left.
- A click on the clock opens the Calendar page by default
  (`clockClick: calendar`); `pin` is still there.
- The open view and the glance are only built while they're shown and let
  go a moment after, so a closed island holds no pages, cover art or icons
  (shell PSS with the island closed: 285 → 278 MB).
- The open view is built in the background as soon as the pointer reaches
  the clock, the world clock or the weather, and opening straight onto a
  page loads it over a few frames: the first visible frame comes about
  24 ms after the island starts opening, with no frame over 25 ms.
- The track position only ticks while music plays and the island is open;
  the bar widget looks for the clock less often while the island is
  closed.

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

[0.6.0]: https://github.com/thealejandro/omarchy-island/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/thealejandro/omarchy-island/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/thealejandro/omarchy-island/releases/tag/v0.4.0
