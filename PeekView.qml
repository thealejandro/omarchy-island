import QtQuick
import qs.Commons
import qs.Commons as Commons
import "IslandModel.js" as IslandModel

// The glance: one line saying what just changed.
Row {
  id: root

  required property var island

  readonly property bool showTimer: island.peekReason === "timer" || (!island.hasMedia && island.timerActive)
  readonly property int thumbSize: Style.space(22)

  spacing: Style.spacing.lg
  height: thumbSize

  Rectangle {
    anchors.verticalCenter: parent.verticalCenter
    width: root.thumbSize
    height: root.thumbSize
    radius: Style.space(6)
    color: Util.alpha(Commons.Color.bar.text, 0.08)
    clip: true

    Image {
      anchors.fill: parent
      visible: !root.showTimer && status === Image.Ready
      source: root.showTimer ? "" : root.island.artUrl
      sourceSize: Qt.size(64, 64)
      fillMode: Image.PreserveAspectCrop
      asynchronous: true
    }
    Text {
      anchors.centerIn: parent
      visible: root.showTimer || root.island.artUrl === ""
      text: root.showTimer ? "󰔛" : "󰝚"
      color: Commons.Color.accent
      font.family: Style.font.family
      font.pixelSize: Style.font.icon
    }
  }

  Text {
    anchors.verticalCenter: parent.verticalCenter
    width: Math.min(implicitWidth, Style.space(320))
    elide: Text.ElideRight
    textFormat: Text.PlainText
    text: root.showTimer
      ? root.island.tr(root.island.timerPhase) + "  " + IslandModel.clock(root.island.timerRemaining)
      : root.island.title + (root.island.artist ? "  ·  " + root.island.artist : "")
    color: Commons.Color.bar.text
    font.family: Style.font.family
    font.pixelSize: Style.font.body
    font.bold: true
  }

  Equalizer {
    anchors.verticalCenter: parent.verticalCenter
    visible: !root.showTimer
    playing: root.island.playing
  }
}
