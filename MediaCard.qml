import QtQuick
import Quickshell.Widgets
import qs.Commons
import qs.Commons as Commons
import "IslandModel.js" as IslandModel

// Now playing: cover, title, artist and source, a seek bar and transport.
Column {
  id: root

  required property var island

  readonly property color textColor: Commons.Color.bar.text
  readonly property color dimColor: Util.alpha(Commons.Color.bar.text, 0.6)
  readonly property int artSize: Style.space(76)

  spacing: Style.spacing.xxl

  Row {
    width: parent.width
    spacing: Style.spacing.xxl

    ClippingRectangle {
      id: art
      width: root.artSize
      height: root.artSize
      radius: Style.space(12)
      color: Util.alpha(root.textColor, 0.08)

      Image {
        anchors.fill: parent
        source: root.island.artUrl
        sourceSize: Qt.size(root.artSize * 2, root.artSize * 2)
        fillMode: Image.PreserveAspectCrop
        asynchronous: true
        visible: status === Image.Ready
      }
      Text {
        anchors.centerIn: parent
        visible: root.island.artUrl === ""
        text: "󰝚"
        color: root.dimColor
        font.family: Style.font.family
        font.pixelSize: Style.font.display
      }
    }

    Column {
      width: parent.width - root.artSize - parent.spacing
      anchors.verticalCenter: parent.verticalCenter
      spacing: Style.spacing.sm

      Row {
        spacing: Style.spacing.md
        visible: root.island.source !== ""
        Equalizer {
          anchors.verticalCenter: parent.verticalCenter
          playing: root.island.playing
          barHeight: Style.space(9)
        }
        Text {
          anchors.verticalCenter: parent.verticalCenter
          text: root.island.source
          color: Commons.Color.accent
          font.family: Style.font.family
          font.pixelSize: Style.font.caption
        }
      }
      Text {
        width: parent.width
        elide: Text.ElideRight
        textFormat: Text.PlainText
        text: root.island.hasMedia ? root.island.title : root.island.tr("nothingPlaying")
        color: root.textColor
        font.family: Style.font.family
        font.pixelSize: Style.font.heading
        font.bold: true
      }
      Text {
        width: parent.width
        elide: Text.ElideRight
        textFormat: Text.PlainText
        text: root.island.hasMedia ? root.island.artist : root.island.tr("nothingPlayingHint")
        visible: text !== ""
        color: root.dimColor
        font.family: Style.font.family
        font.pixelSize: Style.font.body
      }
    }
  }

  // Seek bar: click or drag anywhere on it.
  Item {
    width: parent.width
    height: Style.space(16)
    visible: root.island.hasMedia && root.island.length > 0

    readonly property real progress: Math.min(1, root.island.position / Math.max(1, root.island.length))

    Text {
      id: elapsed
      anchors.left: parent.left
      anchors.verticalCenter: parent.verticalCenter
      text: IslandModel.clock(seek.pressed ? seek.dragFraction * root.island.length : root.island.position)
      color: root.dimColor
      font.family: Style.font.family
      font.pixelSize: Style.font.caption
    }
    Item {
      id: track
      anchors.left: elapsed.right
      anchors.right: total.left
      anchors.leftMargin: Style.spacing.lg
      anchors.rightMargin: Style.spacing.lg
      anchors.verticalCenter: parent.verticalCenter
      height: Style.space(4)

      readonly property real shown: seek.pressed ? seek.dragFraction : parent.progress

      Rectangle {
        anchors.fill: parent
        radius: height / 2
        color: Util.alpha(root.textColor, 0.15)
      }
      Rectangle {
        height: parent.height
        width: parent.width * track.shown
        radius: height / 2
        color: Commons.Color.accent
      }
      Rectangle {
        width: Style.space(10)
        height: width
        radius: width / 2
        x: parent.width * track.shown - width / 2
        anchors.verticalCenter: parent.verticalCenter
        color: Commons.Color.accent
        scale: seek.containsMouse || seek.pressed ? 1 : 0
        Behavior on scale { NumberAnimation { duration: 120 } }
      }
      MouseArea {
        id: seek
        property real dragFraction: 0
        anchors.fill: parent
        anchors.topMargin: -Style.space(6)
        anchors.bottomMargin: -Style.space(6)
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        function fractionAt(x) { return Math.max(0, Math.min(1, x / track.width)) }
        onPressed: (mouse) => dragFraction = fractionAt(mouse.x)
        onPositionChanged: (mouse) => { if (pressed) dragFraction = fractionAt(mouse.x) }
        onReleased: (mouse) => root.island.seekTo(fractionAt(mouse.x))
      }
    }
    Text {
      id: total
      anchors.right: parent.right
      anchors.verticalCenter: parent.verticalCenter
      text: IslandModel.clock(root.island.length)
      color: root.dimColor
      font.family: Style.font.family
      font.pixelSize: Style.font.caption
    }
  }

  Row {
    anchors.horizontalCenter: parent.horizontalCenter
    spacing: Style.spacing.huge
    visible: root.island.hasMedia
    IslandButton { anchors.verticalCenter: parent.verticalCenter; glyph: "󰒮"; onClicked: root.island.previous() }
    IslandButton {
      anchors.verticalCenter: parent.verticalCenter
      glyph: root.island.playing ? "󰏤" : "󰐊"
      filled: true
      size: Style.space(38)
      onClicked: root.island.togglePlaying()
    }
    IslandButton { anchors.verticalCenter: parent.verticalCenter; glyph: "󰒭"; onClicked: root.island.next() }
  }
}
