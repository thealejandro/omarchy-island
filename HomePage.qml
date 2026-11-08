import QtQuick
import qs.Commons
import qs.Commons as Commons

// The island's first page: now playing next to the month, focus timer
// underneath, the world clock and a dock of toggles, launchers and the weather along the
// bottom. Each part can be switched off in the settings.
Column {
  id: root

  required property var island
  property bool shown: false

  readonly property var cfg: island.cfg
  readonly property int calendarWidth: Style.space(196)
  readonly property color lineColor: Util.alpha(Commons.Color.bar.text, 0.2)
  readonly property int rise: Style.space(10)

  // One clock for the whole entrance. Each section starts a little after the
  // one above it, so the content cascades down as the body settles.
  property real progress: shown ? 1 : 0
  Behavior on progress { enabled: root.island.animate; NumberAnimation { duration: root.shown ? 560 : 120 } }

  function reveal(index) {
    var t = Math.max(0, Math.min(1, (progress - index * 0.12) / 0.64))
    return 1 - Math.pow(1 - t, 3)
  }

  spacing: Style.spacing.xl

  Row {
    id: top
    width: parent.width
    spacing: Style.spacing.xxl
    layoutDirection: root.cfg.calendarFirst ? Qt.RightToLeft : Qt.LeftToRight
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
      visible: root.cfg.showMedia
      island: root.island
      width: parent.width - root.calendarWidth - 2 * parent.spacing - 1
      anchors.verticalCenter: parent.verticalCenter
    }

    Rectangle {
      visible: root.cfg.showMedia
      width: 1
      height: parent.height
      color: root.lineColor
    }

    CalendarView {
      island: root.island
      width: root.cfg.showMedia ? root.calendarWidth : top.width
    }
  }

  Rectangle {
    visible: root.cfg.showTimer || root.cfg.showDock
    width: parent.width
    height: 1
    color: root.lineColor
    opacity: root.reveal(1)
    transform: Scale { origin.x: root.width / 2; xScale: root.reveal(1) }
  }

  TimerRow {
    visible: root.cfg.showTimer
    island: root.island
    width: parent.width
    opacity: root.reveal(2)
    transform: Translate { y: (1 - root.reveal(2)) * root.rise }
  }

  WorldRow {
    island: root.island
    width: parent.width
    opacity: root.reveal(3)
    transform: Translate { y: (1 - root.reveal(3)) * root.rise }
  }

  Dock {
    visible: root.cfg.showDock
    island: root.island
    width: parent.width
    opacity: root.reveal(3)
    transform: Translate { y: (1 - root.reveal(3)) * root.rise }
  }
}
