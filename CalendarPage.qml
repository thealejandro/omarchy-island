import QtQuick
import qs.Commons
import qs.Commons as Commons

// The calendar page: the month on one side with dots on days that have
// something, and the picked day's agenda on the other.
Row {
  id: root

  required property var island

  readonly property color lineColor: Util.alpha(Commons.Color.bar.text, 0.2)

  spacing: Style.spacing.xxl
  height: Math.max(calendar.height, agenda.height)

  CalendarView {
    id: calendar
    island: root.island
    large: true
    width: Math.round(root.width * 0.42)
  }

  Rectangle {
    width: 1
    height: parent.height
    color: root.lineColor
  }

  AgendaList {
    id: agenda
    island: root.island
    width: root.width - calendar.width - 2 * root.spacing - 1
    day: calendar.selected
  }
}
