import QtQuick
import qs.Commons
import qs.Commons as Commons
import "MediaModel.js" as MediaModel

// Only surrounding lines are instantiated; no polling or network here.
Item {
  id: root
  required property var island
  readonly property var rows: MediaModel.lyricRows(island.lyricLines, island.lyricIndex)
  readonly property int focused: rows.findIndex(function(row) { return row.distance === 0 })
  property real shift: 0
  property int revision: 0
  property int previousIndex: -1
  readonly property var focusItem: { revision; return lines.itemAt(focused) }
  readonly property real previousHeight: {
    revision
    var previous = focused > 0 ? lines.itemAt(focused - 1) : null
    return previous ? previous.height + stack.spacing : 0
  }
  clip: true
  Accessible.role: Accessible.StaticText
  Accessible.name: island.tr("a11y_lyric", island.lyricLine)

  Connections {
    target: root.island
    function onLyricIndexChanged() {
      var next = root.island.lyricIndex
      var forward = next === root.previousIndex + 1
      root.previousIndex = next
      scroll.stop()
      root.shift = 0
      if (forward && root.visible && root.island.animate) Qt.callLater(function() {
        root.shift = root.previousHeight
        scroll.restart()
      })
    }
  }
  NumberAnimation {
    id: scroll
    target: root
    property: "shift"
    to: 0
    duration: Style.duration(240)
    easing.type: Easing.OutCubic
  }
  Column {
    id: stack
    width: parent.width
    y: root.height * 0.28 - (root.focusItem ? root.focusItem.y : 0) + root.shift
    spacing: Style.spacing.md
    Repeater {
      id: lines
      model: root.rows
      onItemAdded: root.revision++
      onItemRemoved: root.revision++
      Text {
        required property var modelData
        width: stack.width
        text: modelData.text.trim() !== "" ? modelData.text : "• • •"
        textFormat: Text.PlainText
        wrapMode: Text.Wrap
        maximumLineCount: 3
        elide: Text.ElideRight
        color: Commons.Color.bar.text
        opacity: MediaModel.lyricAlpha(modelData.distance)
        font.family: Style.font.family
        font.pixelSize: Style.font.body
        font.bold: modelData.distance === 0
      }
    }
  }
}
