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
  readonly property int pad: Style.spacing.xxxl
  // An island needs soft corners to read as one; sharp themes still get
  // their square bar, the island just rounds off below it.
  readonly property int radius: Math.max(Style.cornerRadius, Style.space(18))
  readonly property int ear: Style.space(10)
  readonly property int openWidth: Style.space(560)

  // Hot zones: throwing the pointer at the top edge over the bar's center
  // (the notch gesture), resting it on the clock, or just under the bar.
  readonly property int edgeHeight: 2
  readonly property int edgeWidth: Style.space(220)
  readonly property int stripHeight: Style.space(8)
  readonly property int stripWidth: Style.space(320)
  // The bar's clock is its center anchor, so it sits right in the middle.
  readonly property int clockWidth: Style.space(120)

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
  implicitHeight: barSize + Style.space(400)
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
    Region { item: clockZone }
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

  // Covers the clock so hovering it opens the island. Clicks that would have
  // reached the clock are handled here: left pins the island (its calendar
  // stands in for Omarchy's), right and middle are passed on to the clock.
  MouseArea {
    id: clockZone
    x: Math.round((win.width - width) / 2)
    width: win.island.overClock ? win.clockWidth : 0
    height: win.barSize
    acceptedButtons: Qt.LeftButton | Qt.RightButton | Qt.MiddleButton
    cursorShape: Qt.PointingHandCursor
    onPressed: (mouse) => {
      if (mouse.button === Qt.RightButton)
        Quickshell.execDetached(["omarchy-shell", "omarchy.clock", "cycleFormat"])
      else if (mouse.button === Qt.MiddleButton)
        Quickshell.execDetached(["omarchy-menu-timezone"])
      else {
        win.island.pinned = !win.island.pinned
        if (win.island.pinned) {
          enterTimer.stop()
          win.hovered = true
        }
      }
    }
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
    width: Math.max(0, Math.ceil(bodyWidth))
    height: Math.max(0, Math.ceil(bodyHeight))
    x: Math.round((win.width - width) / 2)

    property real bodyWidth: win.targetWidth
    property real bodyHeight: win.targetHeight

    // Springs rather than timed curves: they keep their velocity when the
    // target changes mid-flight, so a quick in-and-out of the pointer bends
    // the motion instead of restarting it. Opening is loose and settles with
    // a little wobble; closing is stiffer and barely overshoots. Width is a
    // touch softer than height so the body stretches before it widens.
    readonly property bool closing: win.mode === "closed"
    Behavior on bodyWidth {
      SpringAnimation {
        spring: body.closing ? 6 : 3.6
        damping: body.closing ? 0.55 : 0.32
        mass: 1
        epsilon: 0.25
      }
    }
    Behavior on bodyHeight {
      SpringAnimation {
        spring: body.closing ? 7 : 4.2
        damping: body.closing ? 0.6 : 0.34
        mass: 1
        epsilon: 0.25
      }
    }

    IslandShape {
      x: -win.ear
      // A spring can dip just below zero while closing.
      bodyWidth: Math.max(0, body.bodyWidth)
      bodyHeight: Math.max(0, body.bodyHeight)
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
        // Drops in from the bar as the body grows; when the island opens
        // further it shrinks back into the content instead of just vanishing.
        readonly property bool shown: win.mode === "peek"
        opacity: shown ? 1 : 0
        visible: opacity > 0
        transformOrigin: Item.Top
        scale: shown ? 1 : (win.mode === "open" ? 0.9 : 0.96)
        transform: Translate {
          y: peekView.shown ? 0 : -Style.space(6)
          Behavior on y { NumberAnimation { duration: 260; easing.type: Easing.OutCubic } }
        }
        Behavior on opacity { NumberAnimation { duration: peekView.shown ? 220 : 140; easing.type: Easing.OutCubic } }
        Behavior on scale { NumberAnimation { duration: 260; easing.type: Easing.OutBack; easing.overshoot: 1.2 } }
      }

      OpenView {
        id: openView
        island: win.island
        x: win.pad
        y: win.pad
        width: win.openWidth - 2 * win.pad
        // Sections come in one after another once the body is about half
        // grown, and everything goes at once on the way out.
        shown: win.mode === "open" && body.bodyHeight > win.targetHeight * 0.45
        opacity: shown ? 1 : 0
        visible: opacity > 0
        Behavior on opacity { NumberAnimation { duration: openView.shown ? 80 : 140 } }
      }
    }
  }
}
