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
  readonly property int artSize: Style.space(64)

  spacing: Style.spacing.xl

  // A new track nudges the cover and slides the titles in.
  readonly property string trackKey: island.title + "\u0000" + island.artist + "\u0000" + island.artUrl
  onTrackKeyChanged: if (visible) trackIn.restart()

  ParallelAnimation {
    id: trackIn
    NumberAnimation { target: art; property: "scale"; from: 0.88; to: 1; duration: 420; easing.type: Easing.OutBack; easing.overshoot: 1.6 }
    NumberAnimation { target: info; property: "opacity"; from: 0; to: 1; duration: 260; easing.type: Easing.OutCubic }
    NumberAnimation { target: infoShift; property: "x"; from: Style.space(10); to: 0; duration: 360; easing.type: Easing.OutCubic }
  }

  Row {
    width: parent.width
    spacing: Style.spacing.xl

    ClippingRectangle {
      id: art
      width: root.artSize
      height: root.artSize
      radius: Style.space(12)
      color: Util.alpha(root.textColor, 0.08)

      Image {
        id: cover
        anchors.fill: parent
        source: root.island.artUrl
        sourceSize: Qt.size(root.artSize * 2, root.artSize * 2)
        fillMode: Image.PreserveAspectCrop
        asynchronous: true
        visible: status === Image.Ready
        // Covers load late; fade them in with a slight zoom instead of
        // popping over the placeholder.
        onStatusChanged: if (status === Image.Ready) coverIn.restart()
        ParallelAnimation {
          id: coverIn
          NumberAnimation { target: cover; property: "opacity"; from: 0; to: 1; duration: 320; easing.type: Easing.OutCubic }
          NumberAnimation { target: cover; property: "scale"; from: 1.12; to: 1; duration: 480; easing.type: Easing.OutCubic }
        }
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
      id: info
      width: parent.width - root.artSize - parent.spacing
      anchors.verticalCenter: parent.verticalCenter
      spacing: Style.spacing.sm
      transform: Translate { id: infoShift }

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
    // The position only updates once a second; gliding between ticks keeps
    // the bar moving instead of stepping. Off while dragging, so it sticks
    // to the pointer.
    property real smooth: progress
    Behavior on smooth {
      enabled: !seek.pressed && !seek.settling
      NumberAnimation { duration: 1000 }
    }
    // After a seek, hold the dropped spot until the player reports it.
    onProgressChanged: if (seek.settling) Qt.callLater(() => seek.settling = false)

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

      readonly property real shown: seek.pressed || seek.settling ? seek.dragFraction : parent.smooth

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
        scale: seek.pressed ? 1.25 : (seek.containsMouse ? 1 : 0)
        Behavior on scale { NumberAnimation { duration: 200; easing.type: Easing.OutBack; easing.overshoot: 2 } }
      }
      MouseArea {
        id: seek
        property real dragFraction: 0
        property bool settling: false
        Timer { id: settleTimeout; interval: 1500; onTriggered: seek.settling = false }
        anchors.fill: parent
        anchors.topMargin: -Style.space(6)
        anchors.bottomMargin: -Style.space(6)
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        function fractionAt(x) { return Math.max(0, Math.min(1, x / track.width)) }
        onPressed: (mouse) => dragFraction = fractionAt(mouse.x)
        onPositionChanged: (mouse) => { if (pressed) dragFraction = fractionAt(mouse.x) }
        onReleased: (mouse) => {
          dragFraction = fractionAt(mouse.x)
          settling = true
          settleTimeout.restart()
          root.island.seekTo(dragFraction)
        }
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
    spacing: Style.spacing.xl
    visible: root.island.hasMedia
    IslandButton { anchors.verticalCenter: parent.verticalCenter; glyph: "󰒮"; onClicked: root.island.previous() }
    IslandButton {
      anchors.verticalCenter: parent.verticalCenter
      glyph: root.island.playing ? "󰏤" : "󰐊"
      filled: true
      size: Style.space(34)
      onClicked: root.island.togglePlaying()
    }
    IslandButton { anchors.verticalCenter: parent.verticalCenter; glyph: "󰒭"; onClicked: root.island.next() }
  }
}
