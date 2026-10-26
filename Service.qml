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
  // done); "pinned" keeps the island open until toggled again over IPC.
  property bool peeking: false
  property bool pinned: false
  property string peekReason: ""

  readonly property int peekDuration: 3500

  // ------------------------------------------------------------- media

  readonly property var players: Mpris.players ? Mpris.players.values : []
  readonly property var player: IslandModel.pickPlayer(players)
  readonly property bool hasMedia: !!player && !!(player.trackTitle || player.trackArtist)
  readonly property bool playing: !!player && player.playbackState === MprisPlaybackState.Playing
  readonly property string title: player ? (player.trackTitle || "") : ""
  readonly property string artist: player ? (player.trackArtist || "") : ""
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

  // Mpris only reports position on seeks, so poll it while something plays.
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
        finished === "focus" ? "Focus done" : "Break over",
        finished === "focus" ? "Take a " + root.breakMinutes + " minute break." : "Ready for another round?"])
      root.peek("timer")
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
    function state(): string {
      return JSON.stringify({ pinned: root.pinned, peeking: root.peeking, playing: root.playing,
        title: root.title, timerRunning: root.timerRunning, timerRemaining: root.timerRemaining })
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
