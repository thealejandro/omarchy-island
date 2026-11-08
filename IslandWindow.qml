import QtQuick
import Quickshell
import Quickshell.Wayland
import qs.Commons
import qs.Commons as Commons
import "IslandGeometry.js" as IslandGeometry

// One strip per screen along the bar's edge, just deep enough for the open
// island. The surface itself is transparent and its input region is only a
// few hot zones while closed, so it never steals clicks from the bar or from
// windows.
//
// Everything is laid out in "along" (with the bar) and "across" (depth from
// the screen edge) terms, so a bar at the bottom or on a side gets the same
// island hanging from it, pointing into the screen.
PanelWindow {
  id: win

  required property var island

  readonly property string edge: island.barPosition
  readonly property bool vertical: IslandGeometry.isVertical(edge)
  readonly property string screenName: screen ? String(screen.name || "") : ""
  readonly property int barSize: island.barHidden ? 0
    : island.barSizeFor(screenName, vertical ? Style.bar.sizeVertical : Style.bar.sizeHorizontal)
  readonly property int pad: Style.spacing.xxxl
  // An island needs soft corners to read as one; sharp themes still get
  // their square bar, the island just rounds off below it.
  readonly property int radius: Math.max(Style.cornerRadius, Style.space(18))
  readonly property int ear: Style.space(10)
  readonly property int openWidth: Style.space(island.openWidth)
  // How far into the screen the window reaches; the open island is capped
  // to it.
  readonly property int maxDepth: vertical ? openWidth + Style.space(24)
    : Math.max(Style.space(300), Math.min(Style.space(640), (screen ? screen.height : 1080) - barSize - Style.space(60)))
  readonly property real screenLength: vertical ? height : width

  // Hot zones: throwing the pointer at the screen edge over the clock (the
  // notch gesture), resting it on the clock, or just past the bar.
  readonly property bool hotZones: island.hoverOpen && !island.barHidden
  readonly property int edgeThickness: 2
  readonly property int stripDepth: Style.space(8)
  readonly property int stripLength: Style.space(320)

  // Where the clock is along the bar, as the island's bar widget found it;
  // the middle of the bar when nothing was reported.
  readonly property var anchorInfo: island.anchorFor(screenName)
  readonly property real clockCenter: anchorInfo ? anchorInfo.center : screenLength / 2
  readonly property real clockLength: anchorInfo ? anchorInfo.length : Style.space(island.clockWidth)

  property bool hovered: false
  readonly property bool expanded: hovered || island.pinned
  readonly property bool peeking: !expanded && island.peeking
  readonly property string mode: expanded ? "open" : (peeking ? "peek" : "closed")
  onModeChanged: {
    if (mode === "open") { keepOpen = true; keepPeek = false }
    else if (mode === "peek") keepPeek = true
    if (mode !== "open") openRelease.restart()
    if (mode !== "peek") peekRelease.restart()
    if (mode === "closed") island.islandClosed()
    else if (mode === "open") island.isOpen = true
  }

  // The views are only built while they're on screen, and let go a moment
  // after they're gone (once the closing motion is over), so a closed
  // island holds no pages, cover art or icons. Reopening within that
  // moment reuses them.
  property bool keepOpen: false
  property bool keepPeek: false
  Timer {
    id: openRelease
    interval: 1500
    onTriggered: {
      win.keepOpen = win.mode === "open"
      // Hand the pages' memory back now rather than at some later sweep.
      if (!win.keepOpen) Qt.callLater(gc)
    }
  }
  Timer { id: peekRelease; interval: 1000; onTriggered: win.keepPeek = win.mode === "peek" }
  readonly property Item openView: openLoader.item
  readonly property Item peekView: peekLoader.item

  readonly property Item activeView: mode === "open" ? openView : (mode === "peek" ? peekView : null)
  readonly property real targetWidth: !activeView ? 0
    : (mode === "open" ? openWidth : Math.min(openWidth, activeView.implicitWidth + 2 * Style.spacing.xxl))
  readonly property real targetHeight: !activeView ? 0
    : Math.min(vertical ? height - Style.space(40) : maxDepth,
      activeView.implicitHeight + 2 * (mode === "open" ? pad : Style.spacing.lg))
  readonly property real targetAlong: vertical ? targetHeight : targetWidth
  readonly property real targetAcross: vertical ? targetWidth : targetHeight

  // The body centers on the clock but never runs off the screen; it glides
  // when a page change resizes it next to a screen corner.
  readonly property real wantedCenter: IslandGeometry.clampCenter(clockCenter,
    Math.max(targetAlong, vertical ? 0 : openWidth) + 2 * ear, screenLength, Style.space(8))
  property real bodyCenter: wantedCenter
  Behavior on bodyCenter { NumberAnimation { duration: 220; easing.type: Easing.OutCubic } }

  function place(alongStart, alongLen, acrossFrom, acrossLen) {
    return IslandGeometry.rect(edge, width, height, alongStart, alongLen, acrossFrom, acrossLen)
  }

  anchors {
    top: win.edge === "top" || win.vertical
    bottom: win.edge === "bottom" || win.vertical
    left: win.edge === "left" || !win.vertical
    right: win.edge === "right" || !win.vertical
  }
  implicitHeight: vertical ? 0 : barSize + maxDepth
  implicitWidth: vertical ? barSize + maxDepth : 0
  color: "transparent"
  exclusionMode: ExclusionMode.Ignore
  WlrLayershell.namespace: "omarchy-island"
  WlrLayershell.layer: WlrLayer.Top
  // Only while a text field in the island is being edited, so the island
  // never takes the keyboard from the window you were typing in.
  WlrLayershell.keyboardFocus: island.wantsKeyboard && expanded ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.None

  // The input region follows the island's *final* size, not the animated
  // one. If it shrank under a still pointer the compositor would send a
  // leave without a matching enter, which is what made the first version
  // snap shut a second after opening. Over the bar only the clock is
  // covered, so its neighbours (world clock, weather...) stay clickable
  // with the island open.
  mask: Region {
    Region { item: edgeZone }
    Region { item: stripZone }
    Region { item: clockZone }
    Region { item: islandZone }
  }

  Item {
    id: edgeZone
    readonly property real len: win.hotZones && win.island.overClock ? win.clockLength + Style.space(100) : 0
    readonly property var r: win.place(win.clockCenter - len / 2, len, 0, len > 0 ? win.edgeThickness : 0)
    x: r.x; y: r.y; width: r.width; height: r.height
  }

  Item {
    id: stripZone
    readonly property real len: win.hotZones ? win.stripLength : 0
    readonly property var r: win.place(win.clockCenter - len / 2, len, win.barSize, len > 0 ? win.stripDepth : 0)
    x: r.x; y: r.y; width: r.width; height: r.height
  }

  // Covers the clock so hovering it opens the island. Clicks that would have
  // reached the clock are handled here: left goes to the island (pin, the
  // calendar page, or Omarchy's own panel, as set), right and middle are
  // passed on to the clock.
  MouseArea {
    id: clockZone
    readonly property real len: win.island.overClock && win.barSize > 0 ? win.clockLength : 0
    readonly property var r: win.place(win.clockCenter - len / 2, len, 0, len > 0 ? win.barSize : 0)
    x: r.x; y: r.y; width: r.width; height: r.height
    acceptedButtons: Qt.LeftButton | Qt.RightButton | Qt.MiddleButton
    cursorShape: Qt.PointingHandCursor
    onPressed: (mouse) => {
      if (mouse.button === Qt.RightButton)
        Quickshell.execDetached(["omarchy-shell", "omarchy.clock", "cycleFormat"])
      else if (mouse.button === Qt.MiddleButton)
        Quickshell.execDetached(["omarchy-menu-timezone"])
      else {
        if (!win.island.clockClicked()) {
          enterTimer.stop()
          win.hovered = false
        } else if (win.island.pinned) {
          enterTimer.stop()
          win.hovered = true
        }
      }
    }
  }

  // Only the island itself, from the bar's inner edge inwards.
  Item {
    id: islandZone
    readonly property bool active: win.mode !== "closed"
    readonly property real len: active ? Math.ceil(win.targetAlong) + 2 * win.ear : 0
    readonly property var r: win.place(win.bodyCenter - len / 2, len, win.barSize, active ? Math.ceil(win.targetAcross) : 0)
    x: r.x; y: r.y; width: r.width; height: r.height
  }

  // A short dwell before opening, so sweeping across the bar doesn't flash
  // the island; leaving gets a grace period so brushing the edge while
  // reaching for a control doesn't snap it shut.
  Timer {
    id: enterTimer
    interval: win.island.hoverDelay
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
        win.island.refreshAnchors()
        if (win.peeking) {
          // Reaching for a notification's glance opens its page.
          if (win.island.peekReason === "notification" && win.island.pages.indexOf("notifications") >= 0)
            win.island.page = "notifications"
          win.hovered = true
        } else enterTimer.restart()
      } else {
        enterTimer.stop()
        // A text field being edited keeps the island up.
        if (!win.island.wantsKeyboard) leaveTimer.restart()
      }
    }
  }

  // Unread notifications while the island is closed: a small dot just
  // past the bar, under the clock. Not part of the input region.
  Rectangle {
    readonly property bool lit: win.mode === "closed" && win.island.cfg.notifyBadge
      && win.island.notificationsLive && win.island.unread > 0 && !win.island.barHidden
    readonly property int d: Style.space(6)
    readonly property var r: win.place(win.clockCenter - d / 2, d, win.barSize + Style.space(3), d)
    x: r.x; y: r.y; width: d; height: d
    radius: d / 2
    color: Commons.Color.accent
    opacity: lit ? 0.9 : 0
    visible: opacity > 0
    Behavior on opacity { NumberAnimation { duration: 180 } }
  }

  Item {
    id: body

    property real bodyWidth: win.targetWidth
    property real bodyHeight: win.targetHeight
    readonly property real along: Math.max(0, win.vertical ? bodyHeight : bodyWidth)
    readonly property real across: Math.max(0, win.vertical ? bodyWidth : bodyHeight)
    readonly property var r: win.place(win.bodyCenter - Math.ceil(along) / 2, Math.ceil(along), win.barSize, Math.ceil(across))
    x: r.x; y: r.y; width: r.width; height: r.height

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
      edge: win.edge
      // A spring can dip just below zero while closing.
      bodyWidth: body.along
      bodyHeight: body.across
      radius: win.radius
      ear: win.ear
      // Same tint as the bar where they meet: Omarchy's glassy bar draws its
      // background at 35% at most and lets Hyprland blur the rest.
      readonly property color bg: Commons.Color.bar.background
      fillColor: win.island.glassy ? Qt.rgba(bg.r, bg.g, bg.b, Math.min(bg.a, 0.35)) : bg
      fillColorEnd: win.island.glassy ? Qt.rgba(bg.r, bg.g, bg.b, Math.min(bg.a, 0.88)) : bg
      fadeLength: Math.max(win.barSize, Style.space(26))
      // Very subtle, and a bit stronger on light bars, where the island would
      // otherwise vanish into the wallpaper. Themes can set it in shell.toml:
      // [island] border = "<color>", border-alpha = 0..1, border-width = px.
      strokeColor: Commons.Color.composed("island.border", "island.border-alpha",
        Commons.Color.bar.text, Commons.Color.bar.background.hslLightness > 0.5 ? 0.16 : 0.08)
      strokeWidth: Number(Commons.Color.pick("island.border-width", "1"))
    }

    Item {
      id: content
      anchors.fill: parent
      clip: true

      // The content keeps its final size and stays pinned to the bar side,
      // so it is uncovered from the bar outwards as the body grows.
      Item {
        id: inner
        width: Math.max(body.width, win.targetWidth)
        height: Math.max(body.height, win.targetHeight)
        x: win.edge === "right" ? body.width - width : 0
        y: win.edge === "bottom" ? body.height - height : 0

        Loader {
          id: peekLoader
          active: win.keepPeek
          sourceComponent: Component {
            PeekView {
              id: peekItem
              island: win.island
              x: Style.spacing.xxl
              y: win.edge === "bottom" ? inner.height - height - Style.spacing.lg : Style.spacing.lg
              width: Math.max(0, body.width - 2 * Style.spacing.xxl)
              // Drops in from the bar as the body grows; when the island opens
              // further it shrinks back into the content instead of just vanishing.
              readonly property bool shown: win.mode === "peek"
              readonly property real drop: shown ? 0 : Style.space(6)
              opacity: shown ? 1 : 0
              visible: opacity > 0
              transformOrigin: win.edge === "bottom" ? Item.Bottom : (win.edge === "left" ? Item.Left
                : (win.edge === "right" ? Item.Right : Item.Top))
              scale: shown ? 1 : (win.mode === "open" ? 0.9 : 0.96)
              transform: Translate {
                x: win.edge === "left" ? -peekItem.drop : (win.edge === "right" ? peekItem.drop : 0)
                y: win.edge === "top" ? -peekItem.drop : (win.edge === "bottom" ? peekItem.drop : 0)
                Behavior on x { NumberAnimation { duration: 260; easing.type: Easing.OutCubic } }
                Behavior on y { NumberAnimation { duration: 260; easing.type: Easing.OutCubic } }
              }
              Behavior on opacity { NumberAnimation { duration: peekItem.shown ? 220 : 140; easing.type: Easing.OutCubic } }
              Behavior on scale { NumberAnimation { duration: 260; easing.type: Easing.OutBack; easing.overshoot: 1.2 } }
            }
          }
        }

        Loader {
          id: openLoader
          active: win.keepOpen
          sourceComponent: Component {
            OpenView {
              id: openItem
              island: win.island
              x: win.pad
              y: win.edge === "bottom" ? inner.height - win.targetHeight + win.pad : win.pad
              width: win.openWidth - 2 * win.pad
              maxHeight: (win.vertical ? win.height - Style.space(40) : win.maxDepth) - 2 * win.pad
              // Sections come in one after another once the body is about half
              // grown, and everything goes at once on the way out.
              shown: win.mode === "open" && (win.vertical ? body.bodyWidth : body.bodyHeight) > win.targetAcross * 0.45
              opacity: shown ? 1 : 0
              visible: opacity > 0
              Behavior on opacity { NumberAnimation { duration: openItem.shown ? 80 : 140 } }
            }
          }
        }
      }
    }
  }
}
