import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Services.Mpris
import "IslandModel.js" as IslandModel

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

  readonly property int peekDuration: 3500

  // ------------------------------------------------------------- settings

  // Omarchy doesn't hand settings to service plugins, so read our own entry
  // from shell.json: { "id": "thealejandro.island", "language": "es" }.
  property var settings: ({})
  readonly property string lang: IslandModel.language(settings.language || Quickshell.env("LANG"))
  readonly property var locale: Qt.locale(lang === "es" ? "es_ES" : "en_US")
  // The island sits over the bar's clock by default: hovering it opens the
  // island and a left click pins it, instead of Omarchy's calendar popup.
  readonly property bool overClock: settings.clock !== false

  function tr(key, arg) { return IslandModel.t(lang, key, arg) }

  FileView {
    path: Quickshell.env("HOME") + "/.config/omarchy/shell.json"
    watchChanges: true
    onFileChanged: reload()
    onLoaded: {
      try {
        var plugins = JSON.parse(text()).plugins || []
        var mine = ({})
        for (var i = 0; i < plugins.length; i++)
          if (plugins[i] && plugins[i].id === "thealejandro.island") mine = plugins[i]
        root.settings = mine
      } catch (e) {
        console.warn("island: could not read shell.json:", e)
      }
    }
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
  Timer {
    interval: 1000
    repeat: true
    running: root.playing
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

  function toggleNightLight() { runToggle(["omarchy-shell", "nightlight", "toggle"]) }
  function toggleDoNotDisturb() { runToggle(["omarchy-shell", "notifications", "toggleDnd"]) }
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
      }
    }
  }

  // ------------------------------------------------------------- visibility

  function peek(reason) {
    peekReason = reason || ""
    peeking = true
    peekTimer.restart()
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
    function timer(): string { root.toggleTimer(); return root.timerRunning ? "running" : "paused" }
    function focus(minutes: int): string {
      return root.setFocusMinutes(minutes) ? String(root.focusMinutes) : "busy"
    }
    function state(): string {
      return JSON.stringify({ pinned: root.pinned, peeking: root.peeking, playing: root.playing,
        title: root.title, position: Math.round(root.position), length: Math.round(root.length),
        timerRunning: root.timerRunning, timerRemaining: root.timerRemaining,
        focusMinutes: root.focusMinutes, lang: root.lang })
    }
    function ping(): string { return "ok" }
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
