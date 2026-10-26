import QtQuick
import qs.Commons
import qs.Commons as Commons
import "IslandModel.js" as IslandModel

// The glance: one line saying what just changed.
Row {
  id: root

  required property var island

  readonly property bool showTimer: island.peekReason === "timer" || (!island.hasMedia && island.timerActive)

  spacing: Style.spacing.lg

  Text {
    anchors.verticalCenter: parent.verticalCenter
    text: root.showTimer ? "󰔛" : (root.island.playing ? "󰎈" : "󰏤")
    color: Commons.Color.accent
    font.family: Style.font.family
    font.pixelSize: Style.font.iconLarge
  }

  Text {
    anchors.verticalCenter: parent.verticalCenter
    width: Math.min(implicitWidth, Style.space(300))
    elide: Text.ElideRight
    textFormat: Text.PlainText
    text: root.showTimer
      ? (root.island.timerPhase === "focus" ? "Focus " : "Break ") + IslandModel.clock(root.island.timerRemaining)
      : root.island.title + (root.island.artist ? "  ·  " + root.island.artist : "")
    color: Commons.Color.bar.text
    font.family: Style.font.family
    font.pixelSize: Style.font.body
    font.bold: true
  }
}
