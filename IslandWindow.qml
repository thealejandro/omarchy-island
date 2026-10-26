import QtQuick
import Quickshell
import Quickshell.Wayland
import qs.Commons
import qs.Commons as Commons

// One full-width strip per screen, just tall enough for the open island. The
// surface itself is transparent and its input region is only a few hot zones
// while closed, so it never steals clicks from the bar or from windows.
PanelWindow {
  id: win

  required property var island

  readonly property int barSize: Style.bar.sizeHorizontal
  readonly property int pad: Style.spacing.panelPadding
  // An island needs soft corners to read as one; sharp themes still get
  // their square bar, the island just rounds off below it.
  readonly property int radius: Math.max(Style.cornerRadius, Style.space(18))
  readonly property int ear: Style.space(10)
  readonly property int openWidth: Style.space(620)

  // Hot zones: throwing the pointer at the top edge over the bar's center
  // (the notch gesture), or resting it just under the bar.
  readonly property int edgeHeight: 2
  readonly property int edgeWidth: Style.space(220)
  readonly property int stripHeight: Style.space(8)
  readonly property int stripWidth: Style.space(320)

  property bool hovered: false
  readonly property bool expanded: hovered || island.pinned
  readonly property bool peeking: !expanded && island.peeking
  readonly property string mode: expanded ? "open" : (peeking ? "peek" : "closed")

  readonly property Item activeView: mode === "open" ? openView : (mode === "peek" ? peekView : null)
  readonly property real targetWidth: !activeView ? 0
    : (mode === "open" ? openWidth : Math.min(openWidth, activeView.implicitWidth + 2 * Style.spacing.xxl))
  readonly property real targetHeight: !activeView ? 0
    : activeView.implicitHeight + 2 * (mode === "open" ? pad : Style.spacing.lg)

  anchors { top: true; left: true; right: true }
  implicitHeight: barSize + Style.space(320)
  color: "transparent"
  exclusionMode: ExclusionMode.Ignore
  WlrLayershell.namespace: "omarchy-island"
  WlrLayershell.layer: WlrLayer.Top
  WlrLayershell.keyboardFocus: WlrKeyboardFocus.None

  // The input region follows the island's *final* size, not the animated
  // one, and always keeps the hot zones and the bar column above the island.
  // If it shrank under a still pointer the compositor would send a leave
  // without a matching enter, which is what made the first version snap shut
  // a second after opening.
  mask: Region {
    Region { item: edgeZone }
    Region { item: stripZone }
    Region { item: islandZone }
  }

  Item {
    id: edgeZone
    x: Math.round((win.width - width) / 2)
    width: win.edgeWidth
    height: win.edgeHeight
  }

  Item {
    id: stripZone
    x: Math.round((win.width - width) / 2)
    y: win.barSize
    width: win.stripWidth
    height: win.stripHeight
  }

  Item {
    id: islandZone
    readonly property bool active: win.mode !== "closed"
    x: Math.round((win.width - width) / 2)
    width: active ? Math.ceil(win.targetWidth) + 2 * win.ear : 0
    height: active ? win.barSize + Math.ceil(win.targetHeight) : 0
  }

  // A short dwell before opening, so sweeping across the top of the screen
  // doesn't flash the island; leaving gets a grace period so brushing the
  // edge while reaching for a control doesn't snap it shut.
  Timer {
    id: enterTimer
    interval: 120
    onTriggered: win.hovered = true
  }
  Timer {
    id: leaveTimer
    interval: 300
    onTriggered: win.hovered = false
  }

  HoverHandler {
    onHoveredChanged: {
      if (hovered) {
        leaveTimer.stop()
        if (win.peeking) win.hovered = true
        else enterTimer.restart()
      } else {
        enterTimer.stop()
        leaveTimer.restart()
      }
    }
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
        duration: win.mode === "closed" ? 220 : 380
        easing.type: win.mode === "closed" ? Easing.InOutQuart : Easing.OutBack
        easing.overshoot: 0.9
      }
    }
    Behavior on bodyHeight {
      NumberAnimation {
        duration: win.mode === "closed" ? 220 : 380
        easing.type: win.mode === "closed" ? Easing.InOutQuart : Easing.OutBack
        easing.overshoot: 0.9
      }
    }

    IslandShape {
      x: -win.ear
      bodyWidth: body.bodyWidth
      bodyHeight: body.bodyHeight
      radius: win.radius
      ear: win.ear
      fillColor: Commons.Color.bar.background
    }

    Item {
      id: content
      anchors.fill: parent
      clip: true

      PeekView {
        id: peekView
        island: win.island
        x: Style.spacing.xxl
        y: Style.spacing.lg
        width: Math.max(0, body.width - 2 * Style.spacing.xxl)
        opacity: win.mode === "peek" ? 1 : 0
        visible: opacity > 0
        Behavior on opacity { NumberAnimation { duration: 160 } }
      }

      OpenView {
        id: openView
        island: win.island
        x: win.pad
        y: win.pad
        width: win.openWidth - 2 * win.pad
        // Fade in once the body has mostly grown, and out right away.
        opacity: win.mode === "open" && body.bodyHeight > win.targetHeight * 0.6 ? 1 : 0
        visible: opacity > 0
        Behavior on opacity { NumberAnimation { duration: 180 } }
      }
    }
  }
}
