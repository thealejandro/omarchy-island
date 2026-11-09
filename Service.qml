import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Hyprland
import Quickshell.Services.Mpris
import Quickshell.Services.UPower
import "IslandModel.js" as IslandModel
import "IslandGeometry.js" as IslandGeometry
import "IslandSettings.js" as IslandSettings
import "AgendaModel.js" as AgendaModel
import "IcsModel.js" as IcsModel
import "NotificationModel.js" as NM

// Headless owner of the island state. One IslandWindow is mounted per screen;
// they all read media and timer state from here, so two monitors never show
// two different songs or two diverging timers.
Item {
  id: root

  property var shell: null
  property var manifest: null

  // "peek" is the short glance after something changes (new song, timer
  // done); "pinned" keeps the island open until toggled again (IPC or a
  // click on the clock).
  property bool peeking: false
  property bool pinned: false
  property string peekReason: ""


  // ------------------------------------------------------------- settings

  // Omarchy doesn't hand settings to service plugins, so read our own entry
  // from shell.json: { "id": "thealejandro.island", "language": "es" }. It
  // lives under "plugins", or in the bar layout once the island is put on the
  // bar to hide Omarchy's indicators.
  property var settings: ({})
  // The same entry with every key validated and defaulted (IslandSettings).
  readonly property var cfg: IslandSettings.normalize(settings)
  // Whether shell.json holds an entry for the island at all; without one
  // there is nowhere to save settings to.
  property bool hasEntry: false
  readonly property var barSections: ["left", "centerBegin", "center", "centerEnd", "right", "tray"]
  // Ids of every widget on the bar, to know whether Omarchy's clock,
  // elsewhen or weather are there before handing a click to them.
  property var onBar: []
  property var elsewhenEntry: null
  property string jsonPosition: "top"
  function isOnBar(id) { return onBar.indexOf(id) !== -1 }

  readonly property string lang: IslandModel.language(cfg.language || Quickshell.env("LANG"))
  readonly property var locale: Qt.locale(lang === "es" ? "es_ES" : "en_US")
  // The island sits over the bar's clock by default: hovering it opens the
  // island and a left click pins it, instead of Omarchy's calendar popup.
  readonly property bool overClock: cfg.clock
  // Where Omarchy's quick toggles live: "island" (here, and the bar only
  // shows the ones that are on), "both", or "bar" (Omarchy's own, untouched).
  readonly property string togglesMode: cfg.toggles
  readonly property bool imperial: cfg.units === "imperial"
  // Take over Omarchy's OSD (volume, brightness, mic...). Off by default:
  // only one plugin can own the "osd" IPC target, so Omarchy's has to be
  // disabled first (omarchy plugin disable omarchy.osd).
  readonly property bool osdEnabled: cfg.osd
  readonly property bool batteryEnabled: cfg.battery
  // Left click on the clock: "pin" the island, open it on the "calendar"
  // page, or leave it to "omarchy"'s own calendar panel.
  readonly property string clockClick: cfg.clockClick
  // What the island hangs from: Omarchy's "clock", its own bar "widget", or
  // the "center" of the bar.
  readonly property string anchorMode: cfg.anchor
  readonly property int openWidth: cfg.openWidth
  readonly property int clockWidth: cfg.clockWidth
  readonly property bool hoverOpen: cfg.hoverOpen
  readonly property int hoverDelay: cfg.hoverDelay
  readonly property int peekDuration: cfg.peekDuration
  readonly property string defaultPage: cfg.defaultPage
  // Motion everywhere in the island; off gives instant changes.
  readonly property bool animate: cfg.animations
  // A text field in the island is being edited: the island takes the
  // keyboard and stays open until it's done.
  property bool wantsKeyboard: false
  onWantsKeyboardChanged: if (!wantsKeyboard && keyboardMode) stopKeyboard(false)

  // ------------------------------------------------------------- keyboard navigation

  // A pinned island can be walked with Tab (setting "keyboard"): it takes
  // the keyboard when clicked, never on hover. `island keyboard` pins it
  // and takes the keyboard straight away (keyboardGrab) until Esc or close.
  readonly property bool keyboardNav: cfg.keyboard
  property bool keyboardGrab: false
  onPinnedChanged: if (!pinned) keyboardGrab = false
  // A text field let go of the keyboard: navigation goes on from the island.
  signal keyboardHome()
  function grabKeyboard() {
    if (!keyboardNav) return false
    pinned = true
    keyboardGrab = true
    keyboardHome()
    return true
  }

  // ------------------------------------------------------------- keyboard mode

  // Omarchy's `notifications keyboard`: the island opens on its
  // notifications and takes the keyboard to walk them, until Esc, a while
  // without keys, or the island closing. keyboardOrder is the page's cards
  // top to bottom (set by the page).
  property bool keyboardMode: false
  property string keyboardKey: ""
  property var keyboardOrder: []
  property var keyboardOrderBefore: []
  onKeyboardOrderChanged: {
    if (keyboardMode) keyboardKey = NM.keepSelection(keyboardOrderBefore, keyboardOrder, keyboardKey)
    keyboardOrderBefore = keyboardOrder
  }
  Timer {
    id: keyboardIdle
    interval: root.cfg.notifyKeyboardIdle
    onTriggered: root.stopKeyboard(true)
  }
  function startKeyboard() {
    if (!notify || notify.rows.length === 0) return false
    showPage("notifications")
    pinned = true
    keyboardMode = true
    wantsKeyboard = true
    if (keyboardOrder.indexOf(keyboardKey) < 0) keyboardKey = keyboardOrder.length > 0 ? keyboardOrder[0] : ""
    keyboardIdle.restart()
    return true
  }
  function stopKeyboard(close) {
    keyboardIdle.stop()
    keyboardMode = false
    keyboardKey = ""
    if (wantsKeyboard) wantsKeyboard = false
    if (close) pinned = false
  }
  // A key from the island's window, by name (see NM.keyCommand).
  function keyboardPress(name) {
    if (!keyboardMode) return false
    var c = NM.keyCommand(name)
    if (!c) return false
    keyboardIdle.restart()
    if (c.command === "exit") { stopKeyboard(true); return true }
    if (c.command === "prev" || c.command === "next") {
      keyboardKey = NM.stepKey(keyboardOrder, keyboardKey, c.command === "next" ? 1 : -1)
      return true
    }
    var key = keyboardKey
    var row = key && notify ? notify.rowFor(key) : null
    if (!row) return true
    if (c.command === "activate") notify.activate(key)
    else if (c.command === "dismiss") {
      // A live one moves to Earlier: go on to its neighbour either way.
      var next = NM.stepKey(keyboardOrder, key, 1)
      if (next === key) next = NM.stepKey(keyboardOrder, key, -1)
      if (notify.dismiss(key) && next !== key) keyboardKey = next
    } else if (c.command === "copy") {
      if (row.codes && row.codes.length > 0) notify.copyCode(key, row.codes[0])
    } else if (c.command === "action") {
      if (row.live && c.n <= row.actions.length && notify.invoke(key, row.actions[c.n - 1].id)) stopKeyboard(true)
    }
    return true
  }

  // ------------------------------------------------------------- saving

  readonly property string pluginId: "thealejandro.island"

  // Through the host's own entry writer (the same one Omarchy's clock uses
  // when cycling formats), applied locally first so the page reacts on the
  // click; the shell.json watcher closes the loop. Without that API, the
  // CLI writes one key at a time.
  function saveSetting(key, value) {
    var entry = IslandSettings.withValue(settings, key, value, pluginId)
    if (caps.persist) {
      writeEntry(entry)
      return
    }
    settings = entry
    var stored = entry[key] === undefined ? IslandSettings.spec(key).def : entry[key]
    Quickshell.execDetached(["omarchy", "bar", "set", pluginId, key, JSON.stringify(stored), "--json"])
  }
  // Back to defaults; the OSD and the notifications are Omarchy's by
  // default, so give back whichever the island had.
  function resetSettings() {
    var hadOsd = osdEnabled
    var hadNotifications = notificationsEnabled
    writeEntry(IslandSettings.withoutOwnKeys(settings, pluginId))
    var giveBack = []
    if (hadOsd) giveBack.push("omarchy.osd")
    if (hadNotifications) giveBack.push("omarchy.notifications")
    function next() {
      if (giveBack.length === 0) return
      var id = giveBack.shift()
      root.runThen(["omarchy", "plugin", "enable", id], function() {
        if (id === "omarchy.notifications") root.writeMarker(false)
        next()
      })
    }
    if (giveBack.length > 0) settleThen(next)
  }
  function writeEntry(entry) {
    settings = entry
    if (!caps.persist) return false
    try { return shell.updateEntryInline(pluginId, entry) } catch (e) {
      console.warn("island: could not save settings:", e)
      return false
    }
  }

  // Settings that need Omarchy to change too. Each command runs to the end
  // before the island writes its own entry, so two writers never race on
  // shell.json.
  property var _afterCommand: null
  Process {
    id: commandRunner
    onExited: {
      var next = root._afterCommand
      root._afterCommand = null
      if (next) root.settleThen(next)
    }
  }
  function runThen(command, next) {
    if (commandRunner.running) return false
    _afterCommand = next || null
    commandRunner.command = command
    commandRunner.running = true
    return true
  }
  // The island as the OSD: Omarchy's has to step aside first, since only
  // one plugin can own the "osd" IPC target. And back.
  function setOsd(on) {
    if (on) runThen(["omarchy", "plugin", "disable", "omarchy.osd"], function() { root.saveSetting("osd", true) })
    else {
      saveSetting("osd", false)
      settleThen(function() { root.runThen(["omarchy", "plugin", "enable", "omarchy.osd"]) })
    }
  }
  // The island as the notification server. Omarchy's plugin goes first
  // (only then does the island load its own, see notificationsLive), and on
  // the way back the island lets go before Omarchy's comes back.
  function setNotifications(mode) {
    if (mode === "island") {
      notifyFailed = false
      _probeTries = 0
      runThen(["omarchy", "plugin", "disable", "omarchy.notifications"], function() {
        root.writeMarker(true)
        root.saveSetting("notifications", "island")
      })
    } else {
      saveSetting("notifications", "off")
      settleThen(function() {
        root.runThen(["omarchy", "plugin", "enable", "omarchy.notifications"], function() { root.writeMarker(false) })
      })
    }
  }
  function placeByClock() {
    runThen(["omarchy", "bar", isOnBar(pluginId) ? "move" : "put", pluginId, "--after", "omarchy.clock"])
  }
  function openConfig() {
    runAction(["sh", "-c", 'command -v omarchy-launch-config-editor >/dev/null && exec omarchy-launch-config-editor "$1" || exec xdg-open "$1"',
      "sh", Quickshell.env("HOME") + "/.config/omarchy/shell.json"])
  }
  // shell.json is re-read after each write; give that a moment before the
  // next writer goes.
  Timer { id: settle; interval: 600; property var next: null; onTriggered: if (next) next() }
  function settleThen(fn) { settle.next = fn; settle.restart() }

  // Tested against Omarchy 4.x; another major gets a quiet note in Settings.
  property string omarchyVersion: ""
  readonly property bool omarchyTested: omarchyVersion === "" || /^4\./.test(omarchyVersion)
  FileView {
    path: "/usr/share/omarchy/version"
    printErrors: false
    onLoaded: root.omarchyVersion = text().trim()
  }
  // The island's own version, from its manifest, for the credit line.
  property string version: ""
  FileView {
    path: String(Qt.resolvedUrl("manifest.json")).replace(/^file:\/\//, "")
    printErrors: false
    onLoaded: { try { root.version = String(JSON.parse(text()).version || "") } catch (e) { } }
  }

  // The bar's look in shell.json: "opaque", "glassy" or "none". A glassy bar
  // is see-through with Hyprland blurring what is behind it, so the island
  // copies that (same tint, and its own blur rule) to look like the same
  // surface instead of an opaque patch hanging from a frosted bar.
  property string barLook: "opaque"
  readonly property bool glassy: barLook === "glassy"

  // The only Hyprland call the island makes; off with "blur": false.
  readonly property bool wantsBlur: glassy && cfg.blur
  property bool _blurApplied: false
  function syncBlur() {
    if (!wantsBlur && !_blurApplied) return
    _blurApplied = wantsBlur
    Quickshell.execDetached(["hyprctl", "eval",
      'hl.layer_rule({ match = { namespace = "^omarchy-island$" }, blur = ' + (wantsBlur ? "true" : "false")
        + ', ignore_alpha = 0.3 })'])
  }

  onWantsBlurChanged: syncBlur()

  // A Hyprland reload (theme change, config edit) drops rules added at runtime.
  Connections {
    target: Hyprland
    function onRawEvent(event) { if (event.name === "configreloaded") root.syncBlur() }
  }

  function tr(key, arg) { return IslandModel.t(lang, key, arg) }

  FileView {
    path: Quickshell.env("HOME") + "/.config/omarchy/shell.json"
    watchChanges: true
    onFileChanged: reload()
    onLoaded: {
      try {
        var config = JSON.parse(text())
        var layout = config.bar && config.bar.layout ? config.bar.layout : {}
        // Every section the bar draws, the same list as Omarchy's
        // Util.barSections, plus the "plugins" list for a service-only entry.
        var entries = (config.plugins || []).slice()
        var placed = []
        for (var s = 0; s < root.barSections.length; s++) {
          var list = Array.isArray(layout[root.barSections[s]]) ? layout[root.barSections[s]] : []
          entries = entries.concat(list)
          for (var j = 0; j < list.length; j++) if (list[j] && list[j].id) placed.push(String(list[j].id))
        }
        var mine = ({})
        var elsewhen = null
        for (var i = 0; i < entries.length; i++) {
          if (!entries[i]) continue
          if (entries[i].id === "thealejandro.island") mine = entries[i]
          else if (entries[i].id === "omarchy.elsewhen") elsewhen = entries[i]
        }
        // A disabled plugin's entry can stay in the layout; it's not on the
        // bar for anything the island links to.
        var disabled = Array.isArray(config.disabledPlugins) ? config.disabledPlugins.map(String) : []
        placed = placed.filter(function(id) { return disabled.indexOf(id) === -1 })
        if (elsewhen && disabled.indexOf("omarchy.elsewhen") !== -1) elsewhen = null
        root.settings = mine
        root.hasEntry = !!mine.id
        root.omarchyNotificationsDisabled = disabled.indexOf("omarchy.notifications") !== -1
        if (!root._configRead) { root._configRead = true; orphanCheck.running = true }
        root.onBar = placed
        root.elsewhenEntry = elsewhen
        var bar = config.bar || {}
        root.barLook = bar.background || (bar.transparent === true ? "none" : "opaque")
        root.jsonPosition = String(bar.position || "top")
      } catch (e) {
        console.warn("island: could not read shell.json:", e)
      }
    }
  }

  // ------------------------------------------------------------- bar & anchors

  // What the host offers, checked once instead of assumed: an Omarchy update
  // that drops or renames one of these degrades that one feature only.
  readonly property var caps: ({
    barState: !!(shell && shell.bar && "position" in shell.bar),
    persist: !!(shell && typeof shell.updateEntryInline === "function")
  })

  // Which edge the bar is on, live from the host, or from shell.json.
  readonly property string barPosition: IslandGeometry.edge(caps.barState ? shell.bar.position : jsonPosition)
  readonly property bool barVertical: IslandGeometry.isVertical(barPosition)
  readonly property bool barHidden: caps.barState ? shell.bar.barHidden === true : false

  // The bar's thickness on a screen: the host knows about notch floors and
  // vertical bars; without it fall back to the theme's sizes.
  function barSizeFor(screenName, fallback) {
    if (caps.barState) {
      var sizes = shell.bar.barSizes
      var size = sizes && sizes[screenName] ? Number(sizes[screenName]) : Number(shell.bar.barSize)
      if (size > 0) return size
    }
    return fallback
  }

  // Where to hang from, per screen: { center, length, kind } along the bar,
  // reported by the island's bar widget (the clock's slot, or its own spot)
  // or by IPC. "*" is used for screens nobody reported for.
  property var anchorMap: ({})
  property var _widgets: []

  function setAnchor(screenName, center, length, kind) {
    var key = String(screenName || "*")
    var old = anchorMap[key]
    var c = Math.round(Number(center)), l = Math.round(Number(length))
    if (isNaN(c) || isNaN(l) || l <= 0) return false
    if (old && old.center === c && old.length === l && old.kind === kind) return true
    var next = ({})
    for (var k in anchorMap) next[k] = anchorMap[k]
    next[key] = { center: c, length: l, kind: String(kind || "ipc") }
    anchorMap = next
    return true
  }
  function clearAnchor(screenName) {
    var key = String(screenName || "*")
    if (!anchorMap[key]) return
    var next = ({})
    for (var k in anchorMap) if (k !== key) next[k] = anchorMap[k]
    anchorMap = next
  }
  function anchorFor(screenName) {
    if (anchorMode === "center") return null
    return anchorMap[screenName] || anchorMap["*"] || null
  }

  // Bar widgets reach this service through bar.shell.serviceFor(), the
  // channel Omarchy's own bar hands third-party widgets.
  function registerWidget(w) {
    if (!w || _widgets.indexOf(w) !== -1) return
    _widgets = _widgets.concat([w])
  }
  function unregisterWidget(w) {
    _widgets = _widgets.filter(function(x) { return x !== w })
  }
  function refreshAnchors() {
    for (var i = 0; i < _widgets.length; i++) {
      try { if (_widgets[i] && typeof _widgets[i].reportAnchor === "function") _widgets[i].reportAnchor() }
      catch (e) { }
    }
  }

  // Left click on the clock. Right and middle stay with Omarchy's clock.
  // Returns false when the click went to Omarchy's own panel, so the
  // window can let go of its hover.
  function clockClicked() {
    if (clockClick === "omarchy" && isOnBar("omarchy.clock")) {
      Quickshell.execDetached(["omarchy-shell", "omarchy.clock", "toggle"])
      return false
    }
    if (clockClick === "calendar") return openOn("calendar")
    pinned = !pinned
    return true
  }

  // Opens the island on `name`, or closes it if it's already up there.
  function openOn(name) {
    if (pinned && page === name) { pinned = false; return true }
    pinned = true
    showPage(name)
    return true
  }

  // Omarchy's world clock and weather next to the clock: where they sit on
  // each screen's bar, as the island's bar widget found them, so a left
  // click on them opens the island's World page instead of their popups.
  readonly property bool barClicks: cfg.barClicks
  readonly property var sideWidgets: [
    { id: "omarchy.elsewhen", page: "world" },
    { id: "omarchy.weather", page: "world" }
  ]
  property var sideMap: ({})
  function setSideZones(screenName, zones) {
    var key = String(screenName || "*")
    var next = ({})
    for (var k in sideMap) next[k] = sideMap[k]
    if (JSON.stringify(next[key] || []) === JSON.stringify(zones || [])) return
    next[key] = zones || []
    sideMap = next
  }
  function sideZonesFor(screenName) {
    if (!barClicks) return []
    return sideMap[screenName] || sideMap["*"] || []
  }

  // Which page of the open island is showing. Goes back to the default one
  // every time the island closes.
  readonly property var pages: ["home", "calendar", "world"]
    .concat(notificationsEnabled ? ["notifications"] : []).concat(["settings"])
  property string page: defaultPage
  onDefaultPageChanged: if (!isOpen) page = defaultPage
  property string tabHint: ""
  // The day picked on the calendar page; "" is today.
  property string calendarDay: ""
  function showPage(name) {
    if (pages.indexOf(name) === -1) return false
    page = name
    return true
  }
  function islandClosed() {
    isOpen = false
    if (peekReason === "notification") peeking = false
    calendarDay = ""
    tabHint = ""
    wantsKeyboard = false
    keyboardGrab = false
    page = defaultPage
  }

  // ------------------------------------------------------------- media

  readonly property var players: Mpris.players ? Mpris.players.values : []
  readonly property var player: IslandModel.pickPlayer(players)
  readonly property bool hasMedia: !!player && !!(player.trackTitle || player.trackArtist)
  readonly property bool playing: !!player && player.playbackState === MprisPlaybackState.Playing
  readonly property string title: player ? (player.trackTitle || "") : ""
  readonly property string artist: player ? (player.trackArtist || "") : ""
  readonly property string source: player ? (player.identity || "") : ""
  readonly property string artUrl: player && player.trackArtUrl ? player.trackArtUrl : ""
  readonly property real position: player && player.positionSupported ? player.position : 0
  readonly property real length: player && player.lengthSupported ? player.length : 0

  function togglePlaying() { if (player && player.canTogglePlaying) player.togglePlaying() }
  function next() { if (player && player.canGoNext) player.next() }
  function previous() { if (player && player.canGoPrevious) player.previous() }
  function seekTo(fraction) {
    if (!player || !player.canSeek || length <= 0) return
    player.position = Math.max(0, Math.min(1, fraction)) * length
  }

  // MprisPlayer.position is computed on read but never notifies on its own;
  // Quickshell's docs say to emit positionChanged() on a timer while playing.
  // Only the open island shows the position; closed, nothing needs it.
  Timer {
    interval: 1000
    repeat: true
    running: root.playing && root.isOpen
    triggeredOnStart: true
    onTriggered: if (root.player) root.player.positionChanged()
  }

  property string _lastTrackKey: ""
  onTitleChanged: trackSettle.restart()
  onArtistChanged: trackSettle.restart()

  // Players publish title and artist in separate updates; wait for both to
  // land before deciding whether this is a genuinely new track.
  Timer {
    id: trackSettle
    interval: 150
    onTriggered: {
      var key = root.title + "\u0001" + root.artist
      if (key === root._lastTrackKey) return
      var first = root._lastTrackKey === ""
      root._lastTrackKey = key
      if (!first && root.hasMedia) root.peek("track")
    }
  }

  // ------------------------------------------------------------- focus timer

  property int focusMinutes: 25
  property int breakMinutes: 5
  property string timerPhase: "focus"     // "focus" | "break"
  property bool timerRunning: false
  property int timerRemaining: focusMinutes * 60
  readonly property int timerTotal: (timerPhase === "focus" ? focusMinutes : breakMinutes) * 60
  readonly property bool timerActive: timerRunning || timerRemaining !== timerTotal

  function toggleTimer() { timerRunning = !timerRunning }
  function resetTimer() {
    timerRunning = false
    timerPhase = "focus"
    timerRemaining = focusMinutes * 60
  }

  // Only while idle, so a running session never changes length under you.
  function setFocusMinutes(minutes) {
    if (timerActive) return false
    focusMinutes = Math.max(5, Math.min(120, Math.round(minutes / 5) * 5))
    timerRemaining = timerTotal
    return true
  }

  Timer {
    interval: 1000
    repeat: true
    running: root.timerRunning
    onTriggered: {
      if (root.timerRemaining > 1) {
        root.timerRemaining -= 1
        return
      }
      var finished = root.timerPhase
      root.timerPhase = finished === "focus" ? "break" : "focus"
      root.timerRemaining = root.timerTotal
      root.timerRunning = finished === "focus"
      Quickshell.execDetached(["notify-send", "-a", "Island",
        finished === "focus" ? root.tr("focusDone") : root.tr("breakOver"),
        finished === "focus" ? root.tr("takeBreak", root.breakMinutes) : root.tr("readyAgain")])
      root.peek("timer")
    }
  }

  // ------------------------------------------------------------- quick toggles

  // Omarchy only reveals its inactive bar indicators while the pointer is on
  // the bar's center, which the island now owns, so the island carries them.
  // Third-party plugins can't reach Omarchy's services directly; everything
  // goes through the same IPC and commands the indicators use.
  property bool nightLight: false
  property bool doNotDisturb: false
  property bool stayAwake: false
  property bool recording: false
  property int reminderCount: 0
  property var omarchyReminders: []

  function toggleNightLight() { runToggle(["omarchy-shell", "nightlight", "toggle"]) }
  function toggleDoNotDisturb() {
    // The island's own server answers directly instead of over IPC.
    if (notify) { doNotDisturb = notify.setDnd(!notify.doNotDisturb); return }
    runToggle(["omarchy-shell", "notifications", "toggleDnd"])
  }
  function toggleStayAwake() { runToggle(["omarchy-shell", "idle", "toggle"]) }
  function toggleRecording() {
    runToggle(recording ? ["omarchy-capture-screenrecording", "--stop-recording"]
      : ["omarchy-menu", "toggle", "trigger.capture.screenrecord"])
  }
  function openReminders() {
    runToggle(reminderCount > 0 ? ["omarchy-reminder", "show"] : ["omarchy-reminder", "-i"])
  }

  function runToggle(command) {
    Quickshell.execDetached(command)
    toggleSettle.restart()
  }
  function refreshToggles() { if (!toggleState.running) toggleState.running = true }

  Timer {
    id: toggleSettle
    interval: 500
    onTriggered: root.refreshToggles()
  }

  // One line per toggle, in a fixed order, so a slow command can't shuffle
  // them. pgrep prints a count and exits 1 when nothing matches.
  Process {
    id: toggleState
    command: ["bash", "-c", "omarchy-shell nightlight status; omarchy-shell notifications isDnd; "
      + "omarchy-shell idle status; pgrep -c -f '^gpu-screen-recorder'; omarchy-reminder show --json"]
    stdout: StdioCollector {
      onStreamFinished: {
        var lines = text.split("\n")
        function json(i) { try { return JSON.parse(lines[i]) } catch (e) { return {} } }
        root.nightLight = json(0).enabled === true
        root.doNotDisturb = String(lines[1]).trim() === "on"
        root.stayAwake = json(2).stayAwake === true
        root.recording = Number(lines[3]) > 0
        root.reminderCount = Number(json(4).count || 0)
        root.omarchyReminders = AgendaModel.parseOmarchyReminders(json(4))
      }
    }
  }

  // ------------------------------------------------------------- actions

  // Launchers for Omarchy's tools. The island lets go first and the command
  // waits a moment, so a screenshot or the picker doesn't catch the island.
  function runAction(command) {
    pinned = false
    Quickshell.execDetached(["sh", "-c", "sleep 0.4; exec \"$@\"", "sh"].concat(command))
  }
  function screenshot() { runAction(["omarchy-capture-screenshot"]) }
  function pickColor() { runAction(["hyprpicker", "-a"]) }
  function openClipboard() { runAction(["omarchy-menu-clipboard"]) }
  function openEmoji() { runAction(["omarchy-menu-emoji"]) }
  function openWeather() { runAction(["omarchy-shell", "omarchy.weather", "toggle"]) }

  // ------------------------------------------------------------- weather

  // Current conditions from wttr.in, at the place Omarchy's weather widget
  // was set to (weather.json) or by IP. Refreshed every 20 minutes and kept
  // on failure so the island never flashes an empty pill.
  property var weather: null
  property string weatherQuery: ""
  property string weatherLang: ""
  readonly property string weatherGlyph: weather
    ? IslandModel.weatherIcon(weather.code, weather.hour < 6 || weather.hour >= 19) : ""

  function refreshWeather() {
    if (weatherFetch.running) return
    weatherLang = lang
    weatherFetch.command = ["curl", "-fsS", "--max-time", "8",
      "https://wttr.in/" + weatherQuery + "?format=j1&lang=" + lang]
    weatherFetch.running = true
  }

  FileView {
    path: Quickshell.env("HOME") + "/.local/state/omarchy/settings/weather.json"
    printErrors: false
    watchChanges: true
    onFileChanged: reload()
    onLoaded: {
      try {
        var place = JSON.parse(text())
        var lat = parseFloat(place.latitude), lon = parseFloat(place.longitude)
        root.weatherQuery = !isNaN(lat) && !isNaN(lon) ? lat + "," + lon
          : encodeURIComponent(String(place.name || "").trim())
      } catch (e) {
        root.weatherQuery = ""
      }
      root.refreshWeather()
    }
    onLoadFailed: { root.weatherQuery = ""; root.refreshWeather() }
  }

  // The first fetch can race the settings file; ask again in the right
  // language once it's known.
  onLangChanged: refreshWeather()

  Process {
    id: weatherFetch
    onExited: if (root.weatherLang !== root.lang) root.refreshWeather()
    stdout: StdioCollector {
      onStreamFinished: {
        if (!IslandModel.parseWeather(text, root.lang, root.imperial)) return
        root._weatherRaw = text
        root.parseWeatherRaw()
      }
    }
  }

  // The last good report as wttr.in sent it, so switching units doesn't
  // need a new request.
  property string _weatherRaw: ""
  function parseWeatherRaw() {
    var report = IslandModel.parseWeather(_weatherRaw, lang, imperial)
    if (!report) return
    report.hour = new Date().getHours()
    weather = report
  }
  onImperialChanged: parseWeatherRaw()

  readonly property bool weatherOnBar: isOnBar("omarchy.weather")
  function openForecast() {
    if (weatherOnBar) runAction(["omarchy-shell", "omarchy.weather", "toggle"])
  }
  // Omarchy's weather widget has its own place picker; without it on the
  // bar, the file it reads is opened instead.
  function changeWeatherPlace() {
    if (weatherOnBar) runAction(["omarchy-shell", "omarchy.weather", "edit"])
    else runAction(["sh", "-c", 'command -v omarchy-launch-config-editor >/dev/null && exec omarchy-launch-config-editor "$1" || exec xdg-open "$1"',
      "sh", Quickshell.env("HOME") + "/.local/state/omarchy/settings/weather.json"])
  }

  // ------------------------------------------------------------- world

  // The cities of Omarchy's elsewhen widget (its entry in shell.json, read
  // only), or the island's own list when it has one. No cities, no world.
  readonly property var worldCities: cfg.cities.length > 0 ? cfg.cities
    : (elsewhenEntry ? IslandSettings.parseCities(elsewhenEntry.cities !== undefined ? elsewhenEntry.cities : elsewhenEntry.zones) : [])
  readonly property bool elsewhenOnBar: isOnBar("omarchy.elsewhen")
  // The island's setting, else elsewhen's, else what the locale does.
  readonly property bool hour24: cfg.hour24 !== null ? cfg.hour24
    : (elsewhenEntry && (elsewhenEntry.hour24 === true || elsewhenEntry.hour24 === false) ? elsewhenEntry.hour24
      : String(locale.timeFormat(Locale.ShortFormat)).toLowerCase().indexOf("a") === -1)
  property var zoneProbe: ({})
  property real worldNow: Date.now()
  // True while some screen has the island open; the world clock only ticks
  // then.
  property bool isOpen: false
  readonly property var worldRows: {
    var rows = []
    for (var i = 0; i < worldCities.length; i++) {
      var row = IslandModel.worldRow(worldCities[i], zoneProbe, worldNow, hour24)
      if (row) rows.push(row)
    }
    return rows
  }

  function probeZones() {
    if (worldCities.length === 0 || zoneProbeRun.running) return
    zoneProbeRun.command = IslandModel.zoneProbeCommand(worldCities.map(function(c) { return c.zone }))
    zoneProbeRun.running = true
  }
  onWorldCitiesChanged: probeZones()
  onIsOpenChanged: { if (isOpen) { worldNow = Date.now(); probeZones() }; markSeen() }

  Process {
    id: zoneProbeRun
    stdout: StdioCollector { onStreamFinished: root.zoneProbe = IslandModel.parseZoneProbe(text) }
  }
  // Every minute on the minute while open; offsets again every half hour
  // in case a city changed to or from summer time.
  Timer {
    running: root.isOpen && root.worldCities.length > 0
    repeat: true
    interval: 1000
    property int _probeAt: 0
    onTriggered: {
      var now = Date.now()
      if (Math.floor(now / 60000) !== Math.floor(root.worldNow / 60000)) root.worldNow = now
      if (now - _probeAt > 30 * 60 * 1000) { _probeAt = now; root.probeZones() }
    }
  }

  Timer {
    interval: 20 * 60 * 1000
    running: true
    repeat: true
    onTriggered: root.refreshWeather()
  }

  // ------------------------------------------------------------- agenda

  // The island's own events live in a small file of their own (data, not
  // configuration), written atomically. Omarchy's reminders are listed
  // alongside, and an optional command can add events from elsewhere
  // (khal, gcalcli…), one "YYYY-MM-DD<TAB>HH:MM<TAB>title" per line.
  readonly property string agendaDir: Quickshell.env("HOME") + "/.local/state/omarchy/island"
  property var localEvents: []
  property var externalEvents: []
  // From .ics calendars (icsSources): vdirsyncer folders, files, private
  // iCal links. Read only, like the external ones.
  property var icsEvents: []
  readonly property var allEvents: localEvents.concat(externalEvents, icsEvents)
  // What the agenda peek is about: { title, time, minutes }.
  property var agendaNote: null

  FileView {
    id: eventsFile
    path: root.agendaDir + "/events.json"
    printErrors: false
    atomicWrites: true
    watchChanges: true
    onFileChanged: reload()
    onLoaded: {
      try {
        var data = JSON.parse(text())
        root.localEvents = AgendaModel.cleanAll(Array.isArray(data) ? data : data.events)
      } catch (e) {
        console.warn("island: events.json is not valid JSON; leaving it as it is")
      }
    }
    onLoadFailed: root.localEvents = []
  }

  Process {
    id: agendaDirMaker
    command: ["mkdir", "-p", root.agendaDir]
    property var then: null
    onExited: if (then) { var t = then; then = null; t() }
  }

  function saveEvents(list) {
    var stored = list.map(function(e) {
      return { id: e.id, date: e.date, time: e.time, title: e.title, repeat: e.repeat, remind: e.remind }
    })
    localEvents = AgendaModel.cleanAll(stored)
    var body = JSON.stringify({ version: 1, events: stored }, null, 2) + "\n"
    agendaDirMaker.then = function() { eventsFile.setText(body) }
    agendaDirMaker.running = true
  }

  // fields: { date, time, title, repeat, remind }. Returns the event, or
  // null when it isn't valid (no title, bad date or time).
  function addEvent(fields) {
    var time = AgendaModel.normalizeTime(fields.time)
    if (time === null) return null
    var ev = AgendaModel.clean({ id: AgendaModel.newId(Date.now()), date: fields.date, time: time,
      title: fields.title, repeat: fields.repeat, remind: fields.remind })
    if (!ev) return null
    saveEvents(localEvents.concat([ev]))
    return ev
  }
  function updateEvent(id, fields) {
    var time = AgendaModel.normalizeTime(fields.time)
    if (time === null) return null
    var next = null
    var list = localEvents.map(function(e) {
      if (e.id !== id) return e
      next = AgendaModel.clean({ id: id, date: fields.date, time: time, title: fields.title,
        repeat: fields.repeat, remind: fields.remind })
      return next || e
    })
    if (next) saveEvents(list)
    return next
  }
  function removeEvent(id) {
    saveEvents(localEvents.filter(function(e) { return e.id !== id }))
  }
  function eventsOn(iso) { return AgendaModel.eventsOn(allEvents, iso) }

  // Hands an event of today to Omarchy's reminders, reminderLead minutes
  // before it. False when that moment has already passed.
  function remindViaOmarchy(ev, on) {
    var mins = AgendaModel.minutesUntil(ev, on, new Date())
    if (mins === null) return false
    var lead = ev.remind === null || ev.remind === undefined ? cfg.reminderLead : ev.remind
    var wait = mins - lead
    if (wait < 1) return false
    Quickshell.execDetached(["omarchy-reminder", String(wait), ev.title])
    toggleSettle.restart()
    return true
  }

  // The island's own reminder: a glance and a notification when an event
  // is reminderLead minutes away. Each fires once per day.
  property var _fired: ({})
  Timer {
    interval: 20 * 1000
    repeat: true
    running: root.allEvents.length > 0
    triggeredOnStart: true
    onTriggered: {
      var due = AgendaModel.dueReminders(root.allEvents, new Date(), root.cfg.reminderLead)
      for (var i = 0; i < due.length; i++) {
        if (root._fired[due[i].key]) continue
        root._fired[due[i].key] = true
        var ev = due[i].event
        root.agendaNote = { title: ev.title, time: ev.time, minutes: due[i].minutes }
        Quickshell.execDetached(["notify-send", "-a", "Island", ev.title,
          due[i].minutes > 0 ? root.tr("eventIn", due[i].minutes) + " · " + ev.time : root.tr("eventNow")])
        root.peek("agenda", 8000)
      }
    }
  }

  function refreshExternal() {
    if (cfg.agendaCommand.trim() === "") { externalEvents = []; return }
    if (agendaRun.running) return
    agendaRun.command = ["bash", "-c", cfg.agendaCommand]
    agendaRun.running = true
  }
  Connections {
    target: root
    function onCfgChanged() { if (agendaRun.lastCommand !== root.cfg.agendaCommand) root.refreshExternal() }
  }
  Process {
    id: agendaRun
    property string lastCommand: ""
    onStarted: lastCommand = root.cfg.agendaCommand
    stdout: StdioCollector { onStreamFinished: root.externalEvents = AgendaModel.parseExternal(text) }
  }
  Timer {
    interval: 15 * 60 * 1000
    repeat: true
    running: root.cfg.agendaCommand !== ""
    onTriggered: root.refreshExternal()
  }

  // Calendars: the sources go to the loader as arguments (never through a
  // shell), and its output, calendars plus zone offsets, to IcsModel.
  readonly property var icsSourceList: String(cfg.icsSources || "").split(",")
    .map(function(s) { return s.trim() }).filter(function(s) { return s !== "" })
  function refreshIcs() {
    if (icsSourceList.length === 0) { icsEvents = []; return }
    if (icsRun.running) { icsRun.again = true; return }
    icsRun.command = ["bash", decodeURIComponent(String(Qt.resolvedUrl("scripts/ics-fetch.sh")).replace(/^file:\/\//, ""))]
      .concat(icsSourceList)
    icsRun.running = true
  }
  // Any setting rebuilds the list; only a real change reloads.
  readonly property string icsKey: icsSourceList.join("\n")
  onIcsKeyChanged: refreshIcs()
  Component.onCompleted: refreshIcs()
  Process {
    id: icsRun
    property bool again: false
    stdout: StdioCollector {
      onStreamFinished: {
        root.icsEvents = IcsModel.parse(text, { now: new Date() })
        if (icsRun.again) { icsRun.again = false; Qt.callLater(root.refreshIcs) }
      }
    }
  }
  Timer {
    interval: Math.max(5, root.cfg.icsRefresh) * 60 * 1000
    repeat: true
    running: root.icsSourceList.length > 0
    onTriggered: root.refreshIcs()
  }

  // ------------------------------------------------------------- osd

  // The last OSD Omarchy asked for, shown as a peek instead of its own card.
  property var osd: null

  function showOsd(payloadJson) {
    var next = IslandModel.osdState(payloadJson)
    if (!next) return false
    osd = next
    // The island needs a moment to grow, so short OSDs get a little longer;
    // a zero duration stays up until "osd close".
    peek("osd", next.duration > 0 ? Math.max(1600, next.duration) : 0)
    return true
  }

  IpcHandler {
    target: "osd"
    enabled: root.osdEnabled
    function show(payloadJson: string): string { return root.showOsd(payloadJson) ? "ok" : "invalid" }
    function close(): string { if (root.peekReason === "osd") root.peeking = false; return "ok" }
    function state(): string { return root.peeking && root.peekReason === "osd" ? "open" : "closed" }
    function ping(): string { return "ok" }
  }

  // ------------------------------------------------------------- notifications

  // Off by default. "island" makes the island the desktop's notification
  // server: Omarchy's notifications plugin is disabled (only one server can
  // draw them) and the island shows them as a glance and on their own page.
  readonly property bool notificationsEnabled: cfg.notifications === "island"
  // From shell.json: whether Omarchy's own server has stepped aside.
  property bool omarchyNotificationsDisabled: false
  // The probe never came back; given back to Omarchy for this session.
  property bool notifyFailed: false
  property bool _notifyBounce: false
  property int _probeTries: 0
  property bool _configRead: false
  // Both keys: the setting *and* Omarchy's plugin off. Re-enabling Omarchy's
  // by hand makes the island stand by instead of drawing every
  // notification a second time.
  readonly property bool notificationsLive: notificationsEnabled && omarchyNotificationsDisabled && !notifyFailed
  readonly property bool notificationsStandby: notificationsEnabled && !omarchyNotificationsDisabled
  readonly property var notify: notifyLoader.item
  readonly property int unread: notify ? notify.unread : 0

  Loader {
    id: notifyLoader
    active: root.notificationsLive && !root._notifyBounce
    onActiveChanged: if (active) setSource(Qt.resolvedUrl("NotificationService.qml"), { island: root })
      else setSource("")
    Component.onCompleted: if (active) setSource(Qt.resolvedUrl("NotificationService.qml"), { island: root })
  }

  Connections {
    target: root.notify
    ignoreUnknownSignals: true
    function onDoNotDisturbChanged() { root.doNotDisturb = root.notify.doNotDisturb }
  }

  // The notifications page counts as read once it's on screen.
  function markSeen() { if (notify && isOpen && page === "notifications") notify.markAllRead() }
  onPageChanged: {
    markSeen()
    // Another page ends the keyboard mode, which only walks notifications.
    if (keyboardMode && page !== "notifications") stopKeyboard(false)
  }

  function notifyProbeOk() { _probeTries = 0 }
  // Retried twice by loading the server again; then the notifications go
  // back to Omarchy and Settings says why.
  function notifyProbeFailed() {
    if (_probeTries < 2) {
      _probeTries++
      _notifyBounce = true
      Qt.callLater(function() { root._notifyBounce = false })
      return
    }
    console.warn("island: the notification server did not answer; giving notifications back to Omarchy")
    _probeTries = 0
    notifyFailed = true
    setNotifications("off")
  }

  // A marker for "the island took Omarchy's notifications". If it's there
  // but the setting isn't (reset by hand, reinstall), Omarchy gets them
  // back at startup.
  function writeMarker(on) {
    Quickshell.execDetached(on
      ? ["sh", "-c", 'mkdir -p "$1" && : > "$1/notifications.takeover"', "sh", agendaDir]
      : ["rm", "-f", agendaDir + "/notifications.takeover"])
  }
  // Disabling or removing the island (omarchy plugin disable/remove, bar
  // reset) drops its entry from shell.json, and with it the settings that
  // hand the OSD and the notifications to it: Omarchy would be left with
  // neither, and the island isn't loaded any more to notice. Omarchy has no
  // uninstall hook, and from inside, being torn down for that looks the same
  // as a shell restart; so a detached check looks a moment later, when the
  // plugin folder may already be gone: entry still there, nothing to do;
  // entry gone, Omarchy gets back what the island had.
  readonly property string giveBackScript: '
    sleep 3
    cfg="$HOME/.config/omarchy/shell.json"; id=$1; state=$2; shift 2
    kept=$(jq --arg id "$id" \'[.plugins[]?, (.bar.layout // {} | .[]? | arrays | .[])]
      | any(if type == "object" then .id == $id else . == $id end)\' "$cfg" 2>/dev/null) || exit 0
    [ "$kept" = false ] || exit 0
    for p; do omarchy plugin enable "$p" >/dev/null 2>&1; done
    rm -f "$state/notifications.takeover"
  '
  Component.onDestruction: {
    var giveBack = []
    if (osdEnabled) giveBack.push("omarchy.osd")
    if (notificationsEnabled) giveBack.push("omarchy.notifications")
    if (giveBack.length > 0)
      Quickshell.execDetached(["sh", "-c", giveBackScript, "sh", pluginId, agendaDir].concat(giveBack))
  }
  Process {
    id: orphanCheck
    command: ["test", "-e", root.agendaDir + "/notifications.takeover"]
    onExited: (code) => {
      if (code !== 0 || root.notificationsEnabled) return
      if (!root.omarchyNotificationsDisabled) { root.writeMarker(false); return }
      console.warn("island: notifications were left with the island; giving them back to Omarchy")
      root.runThen(["omarchy", "plugin", "enable", "omarchy.notifications"], function() { root.writeMarker(false) })
    }
  }

  // The same IPC Omarchy's notifications plugin answers (names, arguments
  // and replies), so its menu, scripts and keybindings keep working.
  IpcHandler {
    target: "notifications"
    enabled: root.notificationsLive && !!root.notify
    function ping(): string { return "ok" }
    function isDnd(): string { return root.notify && root.notify.doNotDisturb ? "on" : "off" }
    function toggleDnd(): string { root.toggleDoNotDisturb(); return isDnd() }
    function setDnd(value: string): string {
      var v = String(value || "").toLowerCase()
      if (root.notify) root.doNotDisturb = root.notify.setDnd(v === "true" || v === "1" || v === "on" || v === "yes")
      return isDnd()
    }
    function act(): string { return root.notify && root.notify.actOnNewest() ? "ok" : "none" }
    function dismissOne(): string {
      var key = root.notify ? root.notify.newestLive() : ""
      return key && root.notify.dismiss(key) ? "ok" : "none"
    }
    function dismissAll(): string { if (root.notify) root.notify.dismissAll(); return "ok" }
    function invokeLast(): string {
      var key = root.notify ? root.notify.newestLive() : ""
      return key && root.notify.activate(key) ? "ok" : "none"
    }
    function invokeAction(n: int): string { return root.notify && root.notify.invokeNth(n) ? "ok" : "none" }
    // The island opens on its notifications and takes the keyboard; again
    // gives it back.
    function keyboard(): string {
      if (root.keyboardMode) { root.stopKeyboard(true); return "off" }
      return root.startKeyboard() ? "on" : "none"
    }
    function showHistory(): string { root.showPage("notifications"); root.pinned = true; return "ok" }
    function dismiss(summary: string): string {
      var needle = String(summary || "")
      return needle && root.notify && root.notify.dismissMatching(needle) > 0 ? "ok" : "none"
    }
    function count(): string { return String(root.notify ? root.notify.liveCount : 0) }
    function expand(): string {
      var open = root.pinned && root.page === "notifications"
      if (open) root.pinned = false
      else { root.showPage("notifications"); root.pinned = true }
      return open ? "collapsed" : "expanded"
    }
    function probe(): string {
      var n = root.notify
      return JSON.stringify({ server: "thealejandro.island", toasts: n ? n.liveCount : 0,
        doNotDisturb: !!n && n.doNotDisturb, expanded: root.pinned && root.page === "notifications",
        keyboardMode: root.keyboardMode, keyboardKey: root.keyboardKey, unread: n ? n.unread : 0,
        history: n ? n.historyRows.length : 0, bound: !!n && n.bound })
    }
  }

  // ------------------------------------------------------------- battery

  // Plugging in, unplugging and running low each get a peek. Desktops
  // without a battery never see one.
  readonly property var battery: UPower.displayDevice
  readonly property bool hasBattery: batteryEnabled && !!battery && battery.isPresent && battery.isLaptopBattery
  readonly property int batteryLevel: hasBattery ? Math.round(battery.percentage * 100) : -1
  readonly property bool charging: hasBattery && !UPower.onBattery
  readonly property bool batteryFull: hasBattery && battery.state === UPowerDeviceState.FullyCharged
  property string batteryNote: ""
  property int _lowWarned: 100
  // UPower fills in its state just after startup; that isn't news.
  property bool _batteryReady: false
  Timer { interval: 3000; running: true; onTriggered: root._batteryReady = true }

  onChargingChanged: {
    if (!hasBattery || !_batteryReady) return
    _lowWarned = 100
    batteryNote = charging ? "charging" : "onBattery"
    peek("battery")
  }
  onBatteryFullChanged: if (batteryFull && _batteryReady) { batteryNote = "fullyCharged"; peek("battery") }
  // Warn once at 20% and again at 10% while discharging.
  onBatteryLevelChanged: {
    if (!hasBattery || charging) return
    var mark = batteryLevel <= 10 ? 10 : (batteryLevel <= 20 ? 20 : 100)
    if (mark < _lowWarned && _batteryReady) {
      _lowWarned = mark
      batteryNote = "batteryLow"
      peek("battery")
    }
  }

  // ------------------------------------------------------------- visibility

  // ms overrides how long the peek stays; 0 keeps it until closed.
  function peek(reason, ms) {
    if (!peeking) refreshAnchors()
    peekReason = reason || ""
    peeking = true
    peekTimer.interval = ms === undefined ? peekDuration : ms
    if (peekTimer.interval > 0) peekTimer.restart()
    else peekTimer.stop()
  }

  Timer {
    id: peekTimer
    interval: root.peekDuration
    onTriggered: root.peeking = false
  }

  IpcHandler {
    target: "island"
    function toggle(): string { root.pinned = !root.pinned; return root.pinned ? "open" : "closed" }
    function open(): string { root.pinned = true; return "ok" }
    function close(): string { root.pinned = false; root.peeking = false; return "ok" }
    function peek(): string { root.peek("ipc"); return "ok" }
    // Opens the island and takes the keyboard to walk it with Tab; again
    // (or Esc) lets go and closes it. "off" when the setting is off.
    function keyboard(): string {
      if (root.keyboardGrab) { root.pinned = false; return "closed" }
      return root.grabKeyboard() ? "open" : "off"
    }
    function timer(): string { root.toggleTimer(); return root.timerRunning ? "running" : "paused" }
    function focus(minutes: int): string {
      return root.setFocusMinutes(minutes) ? String(root.focusMinutes) : "busy"
    }
    function state(): string {
      return JSON.stringify({ pinned: root.pinned, peeking: root.peeking, playing: root.playing,
        title: root.title, position: Math.round(root.position), length: Math.round(root.length),
        timerRunning: root.timerRunning, timerRemaining: root.timerRemaining,
        focusMinutes: root.focusMinutes, lang: root.lang, page: root.page, osd: root.osdEnabled,
        battery: root.batteryLevel, charging: root.charging,
        notifications: root.notificationsLive ? "island" : (root.notificationsStandby ? "standby" : "off"),
        unread: root.unread, dnd: root.doNotDisturb, keyboard: root.keyboardGrab })
    }
    function ping(): string { return "ok" }
    // Opens the island on one of its pages: home, calendar, world,
    // notifications (when the island shows them), settings.
    function page(name: string): string {
      if (!root.showPage(name)) return "invalid"
      root.pinned = true
      return "ok"
    }
    // Takes Omarchy's notifications ("island") or gives them back ("off"),
    // the same as the switch in Settings › System.
    function notifications(mode: string): string {
      if (mode !== "island" && mode !== "off") return "invalid"
      root.setNotifications(mode)
      return "ok"
    }
    // Where to hang from on a screen, along the bar, for bars that can't
    // reach the service directly. A length of 0 forgets it.
    function anchor(screen: string, center: int, length: int): string {
      if (length <= 0) { root.clearAnchor(screen); return "ok" }
      return root.setAnchor(screen, center, length, "ipc") ? "ok" : "invalid"
    }
    // Adds an event to the island's agenda: date YYYY-MM-DD, time HH:MM or
    // "" for all day.
    function event(date: string, time: string, title: string): string {
      var ev = root.addEvent({ date: date, time: time, title: title, repeat: "none", remind: null })
      return ev ? ev.id : "invalid"
    }
    function events(date: string): string {
      return JSON.stringify(root.eventsOn(date || AgendaModel.isoDate(new Date())))
    }
    function anchors(): string { return JSON.stringify({ position: root.barPosition, anchors: root.anchorMap, sides: root.sideMap }) }
  }

  Variants {
    model: Quickshell.screens
    IslandWindow {
      required property var modelData
      screen: modelData
      island: root
    }
  }
}
