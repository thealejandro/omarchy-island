import QtQuick
import Quickshell
import qs.Commons
import qs.Commons as Commons
import qs.Ui
import "IslandGeometry.js" as IslandGeometry

// The island's foot on the bar. By default it takes no room; it exists for
// three things:
//
// - The bar API: while the island carries the quick toggles, hovering the
//   bar's center no longer reveals Omarchy's inactive indicators, so the bar
//   keeps just the ones that are on. Omarchy's clock and weather panels clear
//   the flag when they close, so it's put back whenever that happens.
// - Telling the island where to hang from: it finds Omarchy's clock on its
//   own bar (or uses its own spot) and reports that to the service, so the
//   island opens over the clock wherever the clock and the bar are.
// - Optionally (widgetIcon), a button of its own that opens the island.
BarWidget {
  id: root
  moduleName: "thealejandro.island"

  readonly property bool suppress: setting("toggles", "island") === "island"
  readonly property bool suppressed: bar ? bar.centerHoverRevealSuppressed === true : false
  readonly property bool showIcon: String(setting("widgetIcon", false)) === "true"

  // The live service, through the channel Omarchy's own bar hands
  // third-party widgets. A replacement bar gives a service-less facade, and
  // then the anchor goes over IPC instead.
  property var service: null

  visible: showIcon
  implicitWidth: showIcon ? barSize : 0
  implicitHeight: showIcon ? barSize : 0

  function apply() {
    if (bar && typeof bar.setCenterHoverRevealSuppressed === "function" && suppressed !== suppress)
      bar.setCenterHoverRevealSuppressed(suppress)
  }

  function connect() {
    var next = null
    try {
      if (bar && bar.shell && typeof bar.shell.serviceFor === "function")
        next = bar.shell.serviceFor(moduleName)
    } catch (e) {
      next = null
    }
    if (next === service) return
    if (service && typeof service.unregisterWidget === "function") service.unregisterWidget(root)
    service = next && typeof next.registerWidget === "function" ? next : null
    if (service) service.registerWidget(root)
    reportSoon.restart()
  }

  // ---------------------------------------------------------------- anchor

  property var clockItem: null
  property string _lastSent: ""

  function screenName() {
    try {
      var w = root.QsWindow.window
      return w && w.screen ? String(w.screen.name || "") : ""
    } catch (e) {
      return ""
    }
  }

  function windowRoot() {
    var top = root
    for (var i = 0; i < 40 && top.parent; i++) top = top.parent
    return top
  }

  function findClock() {
    try {
      if (clockItem && clockItem.moduleName === "omarchy.clock" && clockItem.width > 0 && clockItem.height > 0)
        return clockItem
    } catch (e) { }
    clockItem = IslandGeometry.findModule(windowRoot(), "omarchy.clock", 4000)
    return clockItem
  }

  function reportAnchor() {
    if (!bar) return
    var mode = service ? service.anchorMode : String(setting("anchor", "clock"))
    var name = screenName()
    var span = null, kind = ""
    if (mode === "center") {
      send(name, 0, 0, "center")
      reportSides(name)
      return
    }
    try {
      if (mode === "clock") {
        var clock = findClock()
        if (clock) { span = IslandGeometry.alongSpan(clock, vertical); kind = "clock" }
      }
      if (!span && root.parent) {
        // Our own place on the bar: right next to the clock when the island
        // was put there with --after omarchy.clock.
        var own = IslandGeometry.alongSpan(root, vertical)
        var len = showIcon ? own.length
          : Style.space(Number(service ? service.clockWidth : setting("clockWidth", 120)) || 120)
        span = { center: own.center, length: len }
        kind = "widget"
      }
    } catch (e) {
      span = null
    }
    if (span) send(name, span.center, span.length, kind)
    reportSides(name)
  }

  // Omarchy's world clock and weather, for the island's click zones.
  property var sideItems: ({})
  function reportSides(name) {
    if (!service || typeof service.setSideZones !== "function") return
    if (!service.barClicks) { service.setSideZones(name, []); return }
    var zones = []
    var list = service.sideWidgets || []
    var cache = sideItems
    for (var i = 0; i < list.length; i++) {
      var id = list[i].id
      if (!service.isOnBar(id)) continue
      var item = null
      try {
        item = cache[id]
        if (!item || item.moduleName !== id || !(item.width > 0 && item.height > 0) || item.visible === false)
          item = IslandGeometry.findModule(windowRoot(), id, 4000)
      } catch (e) { item = null }
      cache[id] = item
      if (!item) continue
      try {
        var span = IslandGeometry.alongSpan(item, vertical)
        if (span && span.length > 0)
          zones.push({ id: id, page: list[i].page, center: Math.round(span.center), length: Math.round(span.length) })
      } catch (e) { }
    }
    sideItems = cache
    service.setSideZones(name, zones)
  }

  function send(name, center, length, kind) {
    var key = name + "|" + Math.round(center) + "|" + Math.round(length) + "|" + kind
    if (service) {
      if (length > 0) service.setAnchor(name || "*", center, length, kind)
      else service.clearAnchor(name || "*")
      _lastSent = key
      return
    }
    if (key === _lastSent || !bar || typeof bar.run !== "function") return
    _lastSent = key
    bar.run("omarchy-shell island anchor " + Util.shellQuote(name || "*") + " "
      + Math.round(center) + " " + Math.round(length))
  }

  // Layout settles over a few frames after a change; the clock also moves
  // when its neighbours change width. Mapping a cached item is cheap.
  Timer {
    id: reportSoon
    interval: 250
    onTriggered: root.reportAnchor()
  }
  // Often while the island is up; a slow check while it's closed, since
  // hovering asks for a fresh report anyway.
  Timer {
    interval: root.service && (root.service.isOpen || root.service.peeking) ? 2000 : 10000
    repeat: true
    running: !!root.bar
    onTriggered: root.reportAnchor()
  }

  Connections {
    target: root.bar
    ignoreUnknownSignals: true
    function onPositionChanged() { root.clockItem = null; root.sideItems = ({}); reportSoon.restart() }
    function onBarSizeChanged() { reportSoon.restart() }
    function onLayoutConfigChanged() { root.clockItem = null; root.sideItems = ({}); reportSoon.restart() }
    function onShellChanged() { root.connect() }
  }

  onSuppressChanged: apply()
  onSuppressedChanged: if (suppress) apply()
  onBarChanged: { apply(); connect() }
  onSettingsChanged: reportSoon.restart()
  Component.onCompleted: { apply(); connect() }
  Component.onDestruction: {
    if (service && typeof service.setSideZones === "function") service.setSideZones(screenName(), [])
    if (suppress && bar) bar.setCenterHoverRevealSuppressed(false)
    if (service) service.unregisterWidget(root)
  }

  // ---------------------------------------------------------------- button

  Text {
    anchors.centerIn: parent
    visible: root.showIcon
    text: "󰕮"
    color: root.bar && root.bar.foreground.a > 0 ? root.bar.foreground : Commons.Color.bar.text
    font.family: Style.font.family
    font.pixelSize: Style.font.icon
  }

  // Unread notifications, when the island shows them.
  Rectangle {
    visible: root.showIcon && !!root.service && root.service.notificationsLive
      && root.service.unread > 0 && root.service.cfg.notifyBadge
    width: Style.space(6)
    height: width
    radius: width / 2
    anchors.right: parent.right
    anchors.top: parent.top
    anchors.margins: Style.space(4)
    color: Commons.Color.accent
  }

  // Microphone and camera in use, top left (the unread dot is top right).
  Row {
    visible: root.showIcon && !!root.service && (root.service.micInUse || root.service.cameraInUse)
    anchors.left: parent.left
    anchors.top: parent.top
    anchors.margins: Style.space(4)
    spacing: Style.space(2)
    Rectangle {
      visible: !!root.service && root.service.micInUse
      width: Style.space(6); height: width; radius: width / 2
      color: Commons.Color.composed("island.mic", "island.mic-alpha", "#ff9f0a", 1)
    }
    Rectangle {
      visible: !!root.service && root.service.cameraInUse
      width: Style.space(6); height: width; radius: width / 2
      color: Commons.Color.composed("island.camera", "island.camera-alpha", "#30d158", 1)
    }
  }

  MouseArea {
    anchors.fill: parent
    enabled: root.showIcon
    cursorShape: Qt.PointingHandCursor
    onClicked: {
      if (root.service) root.service.pinned = !root.service.pinned
      else if (root.bar) root.bar.run("omarchy-shell island toggle")
    }
  }
}
