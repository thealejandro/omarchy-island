import QtQuick
import qs.Commons
import qs.Commons as Commons
import "IslandModel.js" as IslandModel

// The open island: now playing with controls, then the focus timer.
Column {
  id: root

  required property var island

  readonly property color textColor: Commons.Color.bar.text
  readonly property color dimColor: Util.alpha(Commons.Color.bar.text, 0.6)
  readonly property int artSize: Style.space(64)

  spacing: Style.spacing.panelGap

  // ------------------------------------------------------------- media

  Row {
    width: parent.width
    spacing: Style.spacing.xxl

    Rectangle {
      width: root.artSize
      height: root.artSize
      radius: Math.min(Style.cornerRadius, width / 4)
      color: Util.alpha(root.textColor, 0.08)
      clip: true

      Image {
        anchors.fill: parent
        source: root.island.artUrl
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

      Text {
        width: parent.width
        elide: Text.ElideRight
        textFormat: Text.PlainText
        text: root.island.hasMedia ? root.island.title : "Nothing playing"
        color: root.textColor
        font.family: Style.font.family
        font.pixelSize: Style.font.title
        font.bold: true
      }
      Text {
        width: parent.width
        elide: Text.ElideRight
        textFormat: Text.PlainText
        text: root.island.artist
        visible: text !== ""
        color: root.dimColor
        font.family: Style.font.family
        font.pixelSize: Style.font.body
      }

      Row {
        spacing: Style.spacing.xl
        visible: root.island.hasMedia
        IslandButton { glyph: "󰒮"; onClicked: root.island.previous() }
        IslandButton { glyph: root.island.playing ? "󰏤" : "󰐊"; accent: true; onClicked: root.island.togglePlaying() }
        IslandButton { glyph: "󰒭"; onClicked: root.island.next() }
      }
    }
  }

  Item {
    width: parent.width
    height: Style.space(14)
    visible: root.island.hasMedia && root.island.length > 0

    Text {
      id: elapsed
      anchors.left: parent.left
      anchors.verticalCenter: parent.verticalCenter
      text: IslandModel.clock(root.island.position)
      color: root.dimColor
      font.family: Style.font.family
      font.pixelSize: Style.font.caption
    }
    Rectangle {
      id: track
      anchors.left: elapsed.right
      anchors.right: total.left
      anchors.margins: Style.spacing.lg
      anchors.verticalCenter: parent.verticalCenter
      height: Math.max(Style.space(4), Style.spacing.sm)
      color: Util.alpha(root.textColor, 0.2)
      Rectangle {
        height: parent.height
        width: parent.width * Math.min(1, root.island.position / Math.max(1, root.island.length))
        color: Commons.Color.accent
      }
      MouseArea {
        anchors.fill: parent
        anchors.margins: -Style.space(5)
        cursorShape: Qt.PointingHandCursor
        onClicked: (mouse) => root.island.seekTo((mouse.x - Style.space(5)) / track.width)
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

  Rectangle {
    width: parent.width
    height: 1
    color: Util.alpha(root.textColor, 0.12)
  }

  // ------------------------------------------------------------- focus timer

  Item {
    width: parent.width
    height: Math.max(timerText.implicitHeight, timerButtons.height)

    Row {
      anchors.left: parent.left
      anchors.verticalCenter: parent.verticalCenter
      spacing: Style.spacing.lg

      Text {
        anchors.verticalCenter: parent.verticalCenter
        text: "󰔛"
        color: root.island.timerRunning ? Commons.Color.accent : root.dimColor
        font.family: Style.font.family
        font.pixelSize: Style.font.iconLarge
      }
      Text {
        id: timerText
        anchors.verticalCenter: parent.verticalCenter
        text: IslandModel.clock(root.island.timerRemaining)
        color: root.textColor
        font.family: Style.font.family
        font.pixelSize: Style.font.heading
        font.bold: true
      }
      Text {
        anchors.verticalCenter: parent.verticalCenter
        text: root.island.timerPhase === "focus" ? "Focus" : "Break"
        color: root.dimColor
        font.family: Style.font.family
        font.pixelSize: Style.font.body
      }
    }

    Row {
      id: timerButtons
      anchors.right: parent.right
      anchors.verticalCenter: parent.verticalCenter
      spacing: Style.spacing.xl
      IslandButton { glyph: "󰑓"; visible: root.island.timerActive; onClicked: root.island.resetTimer() }
      IslandButton { glyph: root.island.timerRunning ? "󰏤" : "󰐊"; accent: true; onClicked: root.island.toggleTimer() }
    }
  }
}
