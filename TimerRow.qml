import QtQuick
import qs.Commons
import qs.Commons as Commons
import "IslandModel.js" as IslandModel

// Focus timer: phase, remaining time and a thin progress line. Scroll over
// the row to change the focus length while the timer is idle.
Item {
  id: root

  required property var island

  readonly property color dimColor: Util.alpha(Commons.Color.bar.text, 0.6)
  readonly property real progress: island.timerTotal > 0
    ? 1 - island.timerRemaining / island.timerTotal : 0

  implicitHeight: Math.max(left.height, buttons.height) + Style.spacing.md + Style.space(3)

  WheelHandler {
    // Without this the handler ignores a regular mouse wheel.
    acceptedDevices: PointerDevice.Mouse | PointerDevice.TouchPad
    enabled: !root.island.timerActive
    onWheel: (event) => root.island.setFocusMinutes(
      root.island.focusMinutes + (event.angleDelta.y > 0 ? 5 : -5))
  }

  Row {
    id: left
    anchors.left: parent.left
    anchors.top: parent.top
    spacing: Style.spacing.lg

    Text {
      anchors.verticalCenter: parent.verticalCenter
      text: root.island.timerPhase === "focus" ? "󰔛" : "󰾪"
      color: root.island.timerRunning ? Commons.Color.accent : root.dimColor
      font.family: Style.font.family
      font.pixelSize: Style.font.iconLarge
    }
    Text {
      anchors.verticalCenter: parent.verticalCenter
      text: IslandModel.clock(root.island.timerRemaining)
      color: Commons.Color.bar.text
      font.family: Style.font.family
      font.pixelSize: Style.font.display
      font.bold: true
    }
    Column {
      anchors.verticalCenter: parent.verticalCenter
      Text {
        text: root.island.tr(root.island.timerPhase)
        color: Commons.Color.bar.text
        font.family: Style.font.family
        font.pixelSize: Style.font.body
        font.bold: true
      }
      Text {
        text: root.island.timerActive
          ? root.island.tr("minutes", Math.round(root.island.timerTotal / 60))
          : root.island.tr("scrollToChange")
        color: root.dimColor
        font.family: Style.font.family
        font.pixelSize: Style.font.caption
      }
    }
  }

  Row {
    id: buttons
    anchors.right: parent.right
    anchors.verticalCenter: left.verticalCenter
    spacing: Style.spacing.lg
    IslandButton {
      anchors.verticalCenter: parent.verticalCenter
      glyph: "󰑓"
      visible: root.island.timerActive
      onClicked: root.island.resetTimer()
    }
    IslandButton {
      anchors.verticalCenter: parent.verticalCenter
      glyph: root.island.timerRunning ? "󰏤" : "󰐊"
      filled: true
      size: Style.space(30)
      onClicked: root.island.toggleTimer()
    }
  }

  Rectangle {
    anchors.left: parent.left
    anchors.right: parent.right
    anchors.bottom: parent.bottom
    height: Style.space(3)
    radius: height / 2
    color: Util.alpha(Commons.Color.bar.text, 0.1)
    Rectangle {
      height: parent.height
      radius: height / 2
      width: parent.width * root.progress
      color: Commons.Color.accent
      Behavior on width { NumberAnimation { duration: 900 } }
    }
  }
}
