import QtQuick
import Quickshell.Widgets
import qs.Commons
import qs.Commons as Commons
import "IslandModel.js" as IslandModel

// Now playing: cover, title, artist and source, a seek bar and transport.
Column {
  id: root

  required property var island
  property bool inlineLyrics: true

  readonly property color textColor: Commons.Color.bar.text
  readonly property color dimColor: Util.alpha(Commons.Color.bar.text, 0.72)
  readonly property int artSize: Style.space(64)

  spacing: Style.spacing.xl
  function motion(ms) { return island.animate ? Style.duration(ms) : 0 }

  // A new track nudges the cover and slides the titles in.
  readonly property string trackKey: island.title + "\u0000" + island.artist + "\u0000" + island.artUrl
  onTrackKeyChanged: if (visible) trackIn.restart()

  ParallelAnimation {
    id: trackIn
    NumberAnimation { target: art; property: "scale"; from: 0.96; to: 1; duration: root.motion(240); easing.type: Easing.OutCubic }
    NumberAnimation { target: info; property: "opacity"; from: 0; to: 1; duration: root.motion(180); easing.type: Easing.OutCubic }
    NumberAnimation { target: infoShift; property: "x"; from: Style.space(6); to: 0; duration: root.motion(240); easing.type: Easing.OutCubic }
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
          NumberAnimation { target: cover; property: "opacity"; from: 0; to: 1; duration: root.motion(180); easing.type: Easing.OutCubic }
          NumberAnimation { target: cover; property: "scale"; from: 1.04; to: 1; duration: root.motion(240); easing.type: Easing.OutCubic }
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
          animate: root.island.animate
          barHeight: Style.space(9)
        }
        Text {
          anchors.verticalCenter: parent.verticalCenter
          text: root.island.lyricsState === "synced" ? root.island.source + " · " + root.island.tr("lyricsBy") : root.island.source
          textFormat: Text.PlainText
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

  // The synced lyric: the line being sung and the one after it. Both are
  // always single lines, so the box keeps its height and the card doesn't
  // jump between lines. Only there when the song has timed lyrics
  // (lrclib.net, off unless turned on).
  Item {
    id: lyric
    width: parent.width
    height: lyricLines.implicitHeight
    visible: root.inlineLyrics && root.island.cfg.openLyrics && root.island.hasMedia && root.island.lyricsState === "synced"
    clip: true
    Accessible.role: Accessible.StaticText
    Accessible.name: root.island.tr("a11y_lyric", root.island.lyricLine)

    // A new line eases in; with animations off it just changes.
    Connections {
      target: root.island
      function onLyricLineChanged() { if (root.island.animate && lyric.visible) lineIn.restart() }
    }
    ParallelAnimation {
      id: lineIn
      NumberAnimation { target: line; property: "opacity"; from: 0.35; to: 1; duration: root.motion(160); easing.type: Easing.OutCubic }
      NumberAnimation { target: lineShift; property: "y"; from: Style.space(4); to: 0; duration: root.motion(160); easing.type: Easing.OutCubic }
    }

    Column {
      id: lyricLines
      width: parent.width
      spacing: Style.spacing.xs
      Text {
        id: line
        width: parent.width
        elide: Text.ElideRight
        textFormat: Text.PlainText
        // In an instrumental gap there is no line: a quiet ellipsis.
        text: root.island.lyricLine !== "" ? root.island.lyricLine : "…"
        color: root.island.lyricLine !== "" ? root.textColor : root.dimColor
        font.family: Style.font.family
        font.pixelSize: Style.font.body
        transform: Translate { id: lineShift }
      }
      Text {
        width: parent.width
        elide: Text.ElideRight
        textFormat: Text.PlainText
        // Kept a line tall when empty (the last line of the song).
        text: root.island.lyricNext !== "" ? root.island.lyricNext : " "
        color: Util.alpha(root.textColor, 0.5)
        font.family: Style.font.family
        font.pixelSize: Style.font.caption
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
      enabled: root.island.animate && !seek.pressed && !seek.settling
      NumberAnimation { duration: Style.duration(1000) }
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
        Behavior on scale { NumberAnimation { duration: root.motion(160); easing.type: Easing.OutCubic } }
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
    IslandButton { anchors.verticalCenter: parent.verticalCenter; glyph: "󰒮"; label: root.island.tr("a11y_prev"); onClicked: root.island.previous() }
    IslandButton {
      anchors.verticalCenter: parent.verticalCenter
      glyph: root.island.playing ? "󰏤" : "󰐊"
      label: root.island.tr(root.island.playing ? "a11y_pause" : "a11y_play")
      filled: true
      size: Style.space(34)
      onClicked: root.island.togglePlaying()
    }
    IslandButton { anchors.verticalCenter: parent.verticalCenter; glyph: "󰒭"; label: root.island.tr("a11y_next"); onClicked: root.island.next() }
  }
}
