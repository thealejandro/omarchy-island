import QtQuick
import Quickshell.Io
import "LyricsModel.js" as LyricsModel

// Synced lyrics for the song that is playing, from lrclib.net through
// scripts/lyrics-fetch.sh. Loaded only while the "lyrics" setting is on.
//
// A song is looked up only when someone could see its lyrics: the island is
// open, or the lyrics glance setting is on. Once it has lines, one timer
// waits for the next one (never a poll), and each time it fires it reads
// the player's real position again, so it can't drift. With the island
// closed and no glance, nothing runs at all.
Item {
  id: svc

  required property var island

  readonly property string album: island.player ? (island.player.trackAlbum || "") : ""
  readonly property int duration: Math.round(island.length)
  // The song, as far as the player says (the length can arrive later).
  readonly property string trackId: island.title + "\u0001" + island.artist + "\u0001" + album
  readonly property string key: LyricsModel.cacheKey(island.artist, island.title, album, duration)

  // idle | loading | synced | plain | instrumental | none
  property string status: "idle"
  property var lines: []
  property int index: -1
  property string current: ""
  property string next: ""

  // Only synced lyrics can be shown; plain ones are kept (cached) but give
  // no timing, so there is nothing to follow.
  readonly property bool wanted: island.cfg.lyrics && island.hasMedia && status === "synced"
  readonly property bool shown: wanted && ((island.isOpen && island.cfg.openLyrics)
    || (island.cfg.lyricsPeek && island.peeking && island.peekReason === "lyrics"))
  readonly property bool glance: island.cfg.lyricsPeek && wanted && island.playing

  // What was already answered, so pausing, reopening or repeating a song
  // doesn't ask again: the last few results, and the misses until they may
  // be tried again.
  property var memo: ({})
  property var memoOrder: []
  property var misses: ({})
  property var missOrder: []

  function remember(store, order, k, value, max) {
    if (!(k in store)) order.push(k)
    store[k] = value
    while (order.length > max) delete store[order.shift()]
  }

  function reset() {
    tick.stop()
    status = "idle"
    lines = []
    index = -1
    current = ""
    next = ""
  }

  function apply(result) {
    lines = result.kind === "synced" ? result.lines : []
    status = result.kind
  }

  function request() {
    if (status !== "idle" || fetch.running) return
    if (!island.cfg.lyrics || !island.hasMedia) return
    if (island.artist === "" || island.title === "") return
    if ((!island.isOpen || !island.cfg.openLyrics) && !island.cfg.lyricsPeek) return
    // lrclib takes 1-3600 s, and without a length the match isn't reliable.
    if (duration < 10 || duration > 3600) return
    var k = key
    if (memo[k]) { apply(memo[k]); return }
    if (misses[k] && misses[k] > Date.now()) { status = "none"; return }
    status = "loading"
    fetch.reqKey = k
    fetch.reqDuration = duration
    var cleanArtist = LyricsModel.normalizeArtist(island.artist)
    var cleanTitle = LyricsModel.normalizeTitle(island.title)
    fetch.command = ["bash", decodeURIComponent(String(Qt.resolvedUrl("scripts/lyrics-fetch.sh")).replace(/^file:\/\//, "")),
      island.version || "dev", island.artist, island.title, album, String(duration),
      cleanArtist !== island.artist ? cleanArtist : "", cleanTitle !== island.title ? cleanTitle : "", k]
    fetch.running = true
  }

  Process {
    id: fetch
    property string reqKey: ""
    property int reqDuration: 0
    stdout: StdioCollector {
      onStreamFinished: {
        var parsed = LyricsModel.parseFetch(text, fetch.reqDuration)
        var kind = parsed.result ? parsed.result.kind : "none"
        if (parsed.retryMs > 0 || kind === "none")
          svc.remember(svc.misses, svc.missOrder, fetch.reqKey, Date.now() + (parsed.retryMs || 24 * 3600 * 1000), 32)
        else
          svc.remember(svc.memo, svc.memoOrder, fetch.reqKey, parsed.result, 8)
        // The song may have changed while it ran; then it is only remembered
        // and the current one is looked up next.
        if (svc.status === "loading") {
          if (fetch.reqKey === svc.key) svc.apply(parsed.result || { kind: "none", lines: [] })
          else { svc.status = "idle"; ask.restart() }
        } else if (svc.status === "idle") ask.restart()
      }
    }
  }

  // Title, artist and length reach us in separate updates: wait a moment.
  Timer {
    id: ask
    interval: 400
    onTriggered: svc.request()
  }
  onTrackIdChanged: { reset(); ask.restart() }
  onDurationChanged: ask.restart()
  readonly property bool glanceSetting: island.cfg.lyricsPeek
  onGlanceSettingChanged: ask.restart()
  readonly property bool openSetting: island.cfg.openLyrics
  onOpenSettingChanged: ask.restart()

  // ---- following the song

  Timer {
    id: tick
    onTriggered: svc.resync()
  }

  function resync() {
    tick.stop()
    if (!shown) return
    var p = island.player
    var posMs = p && p.positionSupported ? Math.round(p.position * 1000) : 0
    var w = LyricsModel.window(lines, posMs)
    index = w.index
    current = w.current
    next = w.next
    if (island.playing && w.nextAt >= 0) {
      var rate = p && p.rate > 0 ? p.rate : 1
      tick.interval = Math.max(40, Math.round((w.nextAt - posMs) / Math.max(0.25, rate)))
      tick.restart()
    }
  }
  onShownChanged: resync()
  onLinesChanged: resync()
  // A seek (Quickshell emits it on MPRIS Seeked; the open island also does
  // once a second), or a change of speed.
  Connections {
    target: svc.island.player
    function onPositionChanged() { svc.resync() }
    function onRateChanged() { svc.resync() }
  }

  // ---- the glance: the current line stays under the clock

  Timer {
    id: glanceSync
    interval: 300
    onTriggered: {
      if (svc.glance) {
        if (!svc.island.isOpen && !svc.island.peeking) svc.island.peek("lyrics", 0)
      } else {
        svc.releaseGlance()
      }
    }
  }
  // Only ever lets go of its own glance, never someone else's.
  function releaseGlance() {
    if (island.peeking && island.peekReason === "lyrics") island.peeking = false
  }
  onGlanceChanged: glanceSync.restart()

  // The island's own changes, in one place.
  Connections {
    target: svc.island
    function onIsOpenChanged() {
      if (svc.island.isOpen) ask.restart()
      glanceSync.restart()
    }
    function onHasMediaChanged() { ask.restart() }
    function onPlayingChanged() { svc.resync() }
    function onPeekingChanged() { glanceSync.restart() }
  }

  Component.onCompleted: ask.restart()
  Component.onDestruction: releaseGlance()
}
