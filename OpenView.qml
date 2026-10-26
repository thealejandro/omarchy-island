import QtQuick
import qs.Commons
import qs.Commons as Commons

// The open island: now playing next to the month, focus timer underneath.
Column {
  id: root

  required property var island

  readonly property int calendarWidth: Style.space(210)
  readonly property color lineColor: Util.alpha(Commons.Color.bar.text, 0.1)

  spacing: Style.spacing.huge

  Row {
    width: parent.width
    spacing: Style.spacing.huge

    MediaCard {
      island: root.island
      width: parent.width - root.calendarWidth - 2 * parent.spacing - 1
      anchors.verticalCenter: parent.verticalCenter
    }

    Rectangle {
      width: 1
      height: parent.height
      color: root.lineColor
    }

    CalendarView {
      island: root.island
      width: root.calendarWidth
    }
  }

  Rectangle {
    width: parent.width
    height: 1
    color: root.lineColor
  }

  TimerRow {
    island: root.island
    width: parent.width
  }
}
