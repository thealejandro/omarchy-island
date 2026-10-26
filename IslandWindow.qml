import QtQuick
import Quickshell
import Quickshell.Wayland
import qs.Commons
import qs.Commons as Commons

// One full-width strip per screen, just tall enough for the open island. The
// surface itself is transparent and its input region is only the island (or
// a thin hover strip under the bar while closed), so it never steals clicks
// from the bar or from windows.
PanelWindow {
  id: win

  required property var island

  readonly property int barSize: Style.bar.sizeHorizontal
  readonly property int pad: Style.spacing.popupPadding
  readonly property int radius: Style.cornerRadius
  readonly property int openWidth: Style.space(380)
  readonly property int hoverStripHeight: Style.space(6)
  readonly property int hoverStripWidth: Style.space(260)

  property bool hovered: false
  readonly property bool expanded: hovered || island.pinned
  readonly property bool peeking: !expanded && island.peeking
  readonly property string mode: expanded ? "open" : (peeking ? "peek" : "closed")

  readonly property Item activeView: mode === "open" ? openView : (mode === "peek" ? peekView : null)
  readonly property real targetWidth: !activeView ? 0
    : (mode === "open" ? openWidth : Math.min(openWidth, activeView.implicitWidth + 2 * pad))
  readonly property real targetHeight: activeView ? activeView.implicitHeight + 2 * pad : 0

  anchors { top: true; left: true; right: true }
  implicitHeight: barSize + Style.space(260)
  color: "transparent"
  exclusionMode: ExclusionMode.Ignore
  WlrLayershell.namespace: "omarchy-island"
  WlrLayershell.layer: WlrLayer.Top
  WlrLayershell.keyboardFocus: WlrKeyboardFocus.None

  mask: Region { item: win.mode === "closed" ? hoverStrip : body }

  // Leaving the island closes it after a short grace period, so brushing the
  // edge while reaching for a control doesn't snap it shut.
  Timer {
    id: leaveTimer
    interval: 350
    onTriggered: win.hovered = false
  }

  function setHover(h) {
    if (h) { leaveTimer.stop(); win.hovered = true }
    else leaveTimer.restart()
  }

  Item {
    id: hoverStrip
    x: Math.round((win.width - win.hoverStripWidth) / 2)
    y: win.barSize
    width: win.hoverStripWidth
    height: win.hoverStripHeight
    HoverHandler { onHoveredChanged: win.setHover(hovered) }
  }

  Item {
    id: body
    y: win.barSize
    width: Math.ceil(bodyWidth)
    height: Math.ceil(bodyHeight)
    x: Math.round((win.width - width) / 2)

    property real bodyWidth: win.targetWidth
    property real bodyHeight: win.targetHeight

    Behavior on bodyWidth {
      NumberAnimation {
        duration: 320
        easing.type: win.mode === "closed" ? Easing.OutQuart : Easing.OutBack
        easing.overshoot: 1.1
      }
    }
    Behavior on bodyHeight {
      NumberAnimation {
        duration: 320
        easing.type: win.mode === "closed" ? Easing.OutQuart : Easing.OutBack
        easing.overshoot: 1.1
      }
    }

    HoverHandler { onHoveredChanged: win.setHover(hovered) }

    IslandShape {
      x: -e
      bodyWidth: body.bodyWidth
      bodyHeight: body.bodyHeight
      radius: win.radius
      fillColor: Commons.Color.bar.background
    }

    Item {
      anchors.fill: parent
      anchors.margins: win.pad
      clip: true

      PeekView {
        id: peekView
        island: win.island
        width: parent.width
        opacity: win.mode === "peek" ? 1 : 0
        visible: opacity > 0
        Behavior on opacity { NumberAnimation { duration: 160 } }
      }

      OpenView {
        id: openView
        island: win.island
        width: win.openWidth - 2 * win.pad
        opacity: win.mode === "open" ? 1 : 0
        visible: opacity > 0
        Behavior on opacity { NumberAnimation { duration: 200 } }
      }
    }
  }
}
