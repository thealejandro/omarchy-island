import QtQuick
import qs.Commons
import qs.Commons as Commons

// The open island: now playing next to the month, focus timer underneath and
// a dock of toggles, launchers and the weather along the bottom.
Column {
  id: root

  required property var island
  property bool shown: false

  readonly property int calendarWidth: Style.space(196)
  readonly property color lineColor: Util.alpha(Commons.Color.bar.text, 0.1)
  readonly property int rise: Style.space(10)

  // One clock for the whole entrance. Each section starts a little after the
  // one above it, so the content cascades down as the body settles.
  property real progress: shown ? 1 : 0
  Behavior on progress { NumberAnimation { duration: root.shown ? 560 : 120 } }

  function reveal(index) {
    var t = Math.max(0, Math.min(1, (progress - index * 0.12) / 0.64))
    return 1 - Math.pow(1 - t, 3)
  }

  spacing: Style.spacing.xl

  Row {
    id: top
    width: parent.width
    spacing: Style.spacing.xxl
    opacity: root.reveal(0)
    transform: [
      Scale {
        origin.x: top.width / 2
        xScale: 0.96 + 0.04 * root.reveal(0)
        yScale: xScale
      },
      Translate { y: (1 - root.reveal(0)) * root.rise }
    ]

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
    opacity: root.reveal(1)
    transform: Scale { origin.x: root.width / 2; xScale: root.reveal(1) }
  }

  TimerRow {
    island: root.island
    width: parent.width
    opacity: root.reveal(2)
    transform: Translate { y: (1 - root.reveal(2)) * root.rise }
  }

  Dock {
    island: root.island
    width: parent.width
    opacity: root.reveal(3)
    transform: Translate { y: (1 - root.reveal(3)) * root.rise }
  }
}
