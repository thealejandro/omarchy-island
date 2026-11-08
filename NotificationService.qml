import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Services.Notifications
import "NotificationModel.js" as NM

// The island as the desktop's notification server. Only loaded while the
// "notifications" setting is "island" *and* Omarchy's own notifications
// plugin is disabled (Service.qml checks both), so two servers in the same
// shell never draw the same notification twice.
//
// Quickshell keeps the D-Bus name per process: Omarchy's shell has owned
// org.freedesktop.Notifications since it started, and every
// NotificationServer in it hears every notification. With Omarchy's plugin
// gone, this one is the only listener; while it is unloaded nobody would
// show anything, which is why Service.qml checks that this server really
// hears notifications (probe) and gives them back to Omarchy if it doesn't.
Item {
  id: svc

  required property var island
  readonly property var cfg: island.cfg

  // key -> live Notification. Never put these in a model: the object dies
  // with the notification and a dangling role crashes the next read.
  property var refs: ({})
  property int seed: 0
  // Every row, newest first: live ones and the closed ones kept as history.
  property var rows: []
  readonly property var liveRows: rows.filter(function(r) { return r.live })
  readonly property var historyRows: rows.filter(function(r) { return !r.live })
  readonly property int liveCount: liveRows.length
  readonly property int unread: rows.reduce(function(n, r) { return n + (r.read ? 0 : 1) }, 0)
  readonly property var latest: rows.length > 0 ? rows[0] : null

  property bool doNotDisturb: false
  // The probe notification came back: this server really is the one in use.
  property bool bound: false

  // The glance: the row on show, how many more of the same app came in with
  // it, and the keys still waiting.
  property var current: null
  property int burst: 0
  property var queue: []

  NotificationServer {
    id: server
    keepOnReload: false
    bodySupported: true
    bodyMarkupSupported: true
    bodyHyperlinksSupported: true
    bodyImagesSupported: false
    actionsSupported: true
    imageSupported: true
    persistenceSupported: true
    inlineReplySupported: true
    onNotification: (n) => svc.handle(n)
  }

  // ---------------------------------------------------------------- arrival

  function isProbe(n) {
    var h = null
    try { h = n.hints } catch (e) { }
    return (h && h["x-island-probe"] !== undefined)
      || (String(n.appName) === "thealejandro.island" && String(n.summary) === "island-probe")
  }

  function handle(n) {
    if (isProbe(n)) {
      try { n.dismiss() } catch (e) { }
      if (!bound) { bound = true; island.notifyProbeOk() }
      return
    }
    n.tracked = true
    var key = "n" + (++seed)
    refs[key] = n
    var row = NM.snapshot(n, key, Date.now())
    n.closed.connect(function() { svc.closedByServer(key) })
    n.summaryChanged.connect(function() { svc.changedSoon(key) })
    n.bodyChanged.connect(function() { svc.changedSoon(key) })
    arrive(row)
    trimLive()
  }

  function arrive(row) {
    var seen = island.isOpen && island.page === "notifications"
    if (seen) row.read = true
    var next = [row]
    for (var i = 0; i < rows.length; i++) if (rows[i].key !== row.key) next.push(rows[i])
    setRows(next)
    var quiet = doNotDisturb && !NM.bypassesQuiet(row)
    if (!quiet && !island.isOpen) enqueue(row)
  }

  // Too many still open: the oldest are expired, as the spec allows.
  function trimLive() {
    var live = liveRows
    for (var i = NM.LIVE_LIMIT; i < live.length; i++) release(live[i].key, "expire")
  }

  // A sender replacing its notification (same id) changes the object in
  // place; both summary and body usually change, so wait for both.
  property var _changed: ({})
  function changedSoon(key) { _changed[key] = true; changeTimer.restart() }
  Timer {
    id: changeTimer
    interval: 60
    onTriggered: {
      var keys = Object.keys(svc._changed)
      svc._changed = ({})
      for (var i = 0; i < keys.length; i++) {
        var n = svc.refs[keys[i]]
        var old = svc.rowFor(keys[i])
        if (!n || !old) continue
        svc.arrive(NM.refreshed(old, NM.snapshot(n, keys[i], Date.now())))
      }
    }
  }

  // ---------------------------------------------------------------- rows

  function rowFor(key) {
    for (var i = 0; i < rows.length; i++) if (rows[i].key === key) return rows[i]
    return null
  }
  function setRows(next) {
    rows = NM.prune(next, cfg.notifyHistoryLimit, Date.now())
    saveSoon()
  }
  function patch(key, fields) {
    var next = rows.map(function(r) {
      if (r.key !== key) return r
      var c = Object.assign({}, r)
      for (var f in fields) c[f] = fields[f]
      return c
    })
    setRows(next)
  }

  // The sender's object is gone (it closed it, we dismissed it, or it
  // expired): the row stays as history, without anything that needed it.
  function closedByServer(key) {
    if (!refs[key]) return
    delete refs[key]
    var r = rowFor(key)
    if (!r) return
    patch(key, { live: false, actions: [], hasDefault: false, execArgv: null, reply: false,
      image: r.image.indexOf("image://qsimage/") === 0 ? "" : r.image })
    if (current && current.key === key && island.peekReason === "notification") island.peeking = false
  }

  function release(key, how) {
    var n = refs[key]
    if (!n) return false
    try {
      if (how === "expire") n.expire()
      else n.dismiss()
    } catch (e) { }
    closedByServer(key)
    return true
  }

  // ---------------------------------------------------------------- actions

  // Dismissing a live notification keeps it, read, under "Earlier"; a row
  // that is already history goes for good.
  function dismiss(key) {
    var r = rowFor(key)
    if (!r) return false
    if (r.live) { patch(key, { read: true }); return release(key, "dismiss") }
    setRows(rows.filter(function(x) { return x.key !== key }))
    return true
  }
  function dismissAll() {
    var keys = liveRows.map(function(r) { return r.key })
    for (var i = 0; i < keys.length; i++) dismiss(keys[i])
    return keys.length
  }
  function dismissApp(app) {
    var keys = liveRows.filter(function(r) { return NM.appLabel(r) === app }).map(function(r) { return r.key })
    for (var i = 0; i < keys.length; i++) dismiss(keys[i])
  }
  function dismissMatching(needle) {
    var keys = liveRows.filter(function(r) { return r.summary.indexOf(needle) !== -1 }).map(function(r) { return r.key })
    for (var i = 0; i < keys.length; i++) dismiss(keys[i])
    return keys.length
  }
  function clearHistory() { setRows(liveRows) }
  function markAllRead() {
    if (unread === 0) return
    setRows(rows.map(function(r) { return r.read ? r : Object.assign({}, r, { read: true }) }))
  }

  // Click on a notification: the sender's command (omarchy-exec-argv), its
  // default action, or the first safe link in it.
  function activate(key) {
    var r = rowFor(key)
    if (!r) return false
    var n = refs[key]
    var done = false
    if (r.live && r.execArgv) { Quickshell.execDetached(r.execArgv); done = true }
    else if (r.live && r.hasDefault && n) {
      var list = n.actions || []
      for (var i = 0; i < list.length; i++) if (list[i].identifier === "default") { list[i].invoke(); done = true; break }
    }
    if (!done && r.link) { Qt.openUrlExternally(r.link); done = true }
    if (!done) return false
    patch(key, { read: true })
    if (r.live && !r.resident) release(key, "dismiss")
    island.pinned = false
    return true
  }
  function invoke(key, actionId) {
    var n = refs[key]
    if (!n) return false
    var list = n.actions || []
    for (var i = 0; i < list.length; i++) {
      if (list[i].identifier !== actionId) continue
      var resident = n.resident === true
      patch(key, { read: true })
      list[i].invoke()
      if (!resident) closedByServer(key)
      return true
    }
    return false
  }
  function reply(key, text) {
    var n = refs[key]
    var t = String(text || "").trim()
    if (!n || t === "" || typeof n.sendInlineReply !== "function") return false
    patch(key, { read: true })
    n.sendInlineReply(t)
    return true
  }
  function openLink(key) {
    var r = rowFor(key)
    if (!r || !r.link) return false
    Qt.openUrlExternally(r.link)
    patch(key, { read: true })
    island.pinned = false
    return true
  }

  // For the IPC: the newest live one.
  function newestLive() { return liveRows.length > 0 ? liveRows[0].key : "" }
  // One button: press it; a click target: click it; otherwise dismiss.
  function actOnNewest() {
    var key = newestLive()
    if (!key) return false
    var r = rowFor(key)
    if (r.actions.length === 1) return invoke(key, r.actions[0].id)
    if (activate(key)) return true
    return dismiss(key)
  }
  function invokeNth(n) {
    var key = newestLive()
    var r = key ? rowFor(key) : null
    if (!r || n < 1 || n > r.actions.length) return false
    return invoke(key, r.actions[n - 1].id)
  }

  // ---------------------------------------------------------------- glance

  function enqueue(row) {
    // Several from one app in a row read as one glance with a count.
    if (current && current.app === row.app && row.time - current.time < 3000 && island.peekReason === "notification") {
      burst++
      current = row
      island.peek("notification", NM.peekFor(row, cfg.notifyPeekDuration))
      return
    }
    queue = queue.concat([row.key])
    if (!current) showNext()
  }
  function showNext() {
    while (queue.length > 0) {
      var key = queue[0]
      queue = queue.slice(1)
      var r = rowFor(key)
      if (!r || r.read) continue
      current = r
      burst = 0
      island.peek("notification", NM.peekFor(r, cfg.notifyPeekDuration))
      return
    }
    current = null
  }
  Timer { id: nextTimer; interval: 260; onTriggered: if (!svc.island.isOpen) svc.showNext() }

  Connections {
    target: svc.island
    function onPeekingChanged() {
      if (svc.island.peeking) return
      svc.current = null
      if (svc.queue.length > 0) nextTimer.restart()
    }
    // Another glance (a track, the OSD) took over: this one goes back in line.
    function onPeekReasonChanged() {
      if (svc.island.peekReason === "notification" || !svc.current) return
      svc.queue = [svc.current.key].concat(svc.queue)
      svc.current = null
    }
    // Opening the island shows everything there; nothing waits to glance.
    function onIsOpenChanged() {
      if (!svc.island.isOpen) return
      svc.queue = []
      svc.current = null
      if (svc.island.peekReason === "notification") svc.island.peeking = false
    }
  }

  // ---------------------------------------------------------------- quiet

  // Omarchy's own Do Not Disturb file, so the state carries over between
  // the two servers and its menu and scripts read the same thing.
  readonly property string quietDir: Quickshell.env("HOME") + "/.local/state/omarchy/notifications"
  FileView {
    id: quietFile
    path: svc.quietDir + "/quiet.json"
    printErrors: false
    watchChanges: true
    atomicWrites: true
    onFileChanged: reload()
    onLoaded: {
      try { var q = JSON.parse(text()); svc.doNotDisturb = !!q && q.dnd === true } catch (e) { svc.doNotDisturb = false }
    }
    onLoadFailed: svc.doNotDisturb = false
  }
  Process {
    id: quietDirMaker
    property string body: ""
    command: ["mkdir", "-p", svc.quietDir]
    onExited: quietFile.setText(body)
  }
  function setDnd(on) {
    doNotDisturb = !!on
    // The latest press wins if the directory is still being made.
    quietDirMaker.body = JSON.stringify({ dnd: doNotDisturb }) + "\n"
    if (!quietDirMaker.running) quietDirMaker.running = true
    return doNotDisturb
  }

  // ---------------------------------------------------------------- history

  // Closed notifications in ~/.local/state/omarchy/island/notifications.json
  // (0600), written a moment after the last change. Off with notifyHistory.
  readonly property string storePath: island.agendaDir + "/notifications.json"
  property bool storeLoaded: false
  FileView {
    id: storeFile
    path: svc.cfg.notifyHistory ? svc.storePath : ""
    printErrors: false
    atomicWrites: true
    onLoaded: {
      if (svc.storeLoaded) return
      svc.storeLoaded = true
      svc.rows = NM.prune(svc.rows.concat(NM.parseStore(text(), 0)), svc.cfg.notifyHistoryLimit, Date.now())
    }
    onLoadFailed: svc.storeLoaded = true
  }
  Timer { id: saveTimer; interval: 400; onTriggered: svc.flush() }
  function saveSoon() { if (cfg.notifyHistory && storeLoaded) saveTimer.restart() }
  Process {
    id: storeMaker
    property string body: ""
    command: ["sh", "-c", 'umask 077; mkdir -p "$1" && touch "$2" && chmod 600 "$2"', "sh", svc.island.agendaDir, svc.storePath]
    onExited: storeFile.setText(body)
  }
  function flush() {
    if (!cfg.notifyHistory || storeMaker.running) return
    storeMaker.body = NM.serialise(rows)
    storeMaker.running = true
  }
  // History switched off: nothing of it stays on disk.
  Connections {
    target: svc
    function onCfgChanged() {
      if (svc.cfg.notifyHistory) return
      svc.storeLoaded = false
      Quickshell.execDetached(["rm", "-f", svc.storePath])
    }
  }

  // ---------------------------------------------------------------- probe

  // A notification only this server would swallow. If it doesn't come back
  // the island isn't the one hearing them, and Service.qml hands them back
  // to Omarchy rather than leave the desktop silent.
  Timer {
    id: probeSend
    interval: 1200
    running: true
    onTriggered: {
      Quickshell.execDetached(["notify-send", "-a", "thealejandro.island", "-t", "1000",
        "-h", "string:x-island-probe:1", "-h", "boolean:transient:true", "island-probe", "probe"])
      probeWait.start()
    }
  }
  Timer {
    id: probeWait
    interval: 2500
    onTriggered: if (!svc.bound) svc.island.notifyProbeFailed()
  }

  // Live ones can't outlive the server; they go to history unread.
  Component.onDestruction: {
    saveTimer.stop()
    var keys = Object.keys(refs)
    for (var i = 0; i < keys.length; i++) { try { refs[keys[i]].tracked = false } catch (e) { } }
  }
}
