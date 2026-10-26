import QtQuick
import qs.Commons
import qs.Commons as Commons

// Three bouncing bars while something plays; they settle flat when paused.
Row {
  id: root

  property bool playing: false
  property int barHeight: Style.space(12)

  spacing: Style.space(2)
  height: barHeight

  Repeater {
    model: [0.55, 1.0, 0.75]
    Rectangle {
      required property real modelData
      required property int index
      anchors.bottom: parent.bottom
      width: Style.space(3)
      radius: width / 2
      color: Commons.Color.accent
      height: root.barHeight * 0.25

      SequentialAnimation on height {
        running: root.playing
        loops: Animation.Infinite
        alwaysRunToEnd: true
        NumberAnimation { to: root.barHeight * modelData; duration: 260 + index * 70; easing.type: Easing.InOutSine }
        NumberAnimation { to: root.barHeight * 0.25; duration: 300 + index * 50; easing.type: Easing.InOutSine }
      }
    }
  }
}
