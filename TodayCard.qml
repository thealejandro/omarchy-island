import QtQuick
import qs.Commons
import qs.Commons as Commons
import "AgendaModel.js" as AgendaModel

// Home's "what's next": the next few events from now (today's that aren't
// over, then the coming week). A click takes you to that day on the
// Calendar page; with nothing coming up, to the Calendar page to add one.
Column {
  id: root

  required property var island
  property real now: Date.now()

  readonly property color textColor: Commons.Color.bar.text
  readonly property color dimColor: Util.alpha(Commons.Color.bar.text, 0.72)
  readonly property var events: AgendaModel.nextEvents(island.allEvents, new Date(now), 7, 3)
  readonly property string today: AgendaModel.isoDate(new Date(now))
  readonly property string tomorrow: AgendaModel.isoDate(new Date(new Date(now).getFullYear(),
    new Date(now).getMonth(), new Date(now).getDate() + 1))

  spacing: Style.spacing.sm

  Timer {
    interval: 60 * 1000
    repeat: true
    running: root.visible && root.island.isOpen
    onTriggered: root.now = Date.now()
  }
  onVisibleChanged: if (visible) now = Date.now()

  function when(ev) {
    var day = ev.on === today ? "" : (ev.on === tomorrow ? island.tr("tomorrow")
      : island.locale.toString(AgendaModel.parseDate(ev.on), "ddd d"))
    if (ev.on === today && ev.time) {
      var left = AgendaModel.minutesUntil(ev, ev.on, new Date(now))
      if (left !== null && left <= 0) return island.tr("eventNow")
      if (left !== null && left <= 60) return island.tr("eventIn", left)
    }
    var time = ev.time || island.tr("allDay")
    return day ? day + " · " + time : time
  }
  function openDay(iso) {
    island.calendarDay = iso === today ? "" : iso
    island.showPage("calendar")
  }

  Text {
    text: root.island.tr("home_next")
    color: root.dimColor
    font.family: Style.font.family
    font.pixelSize: Style.font.caption
    font.bold: true
    font.capitalization: Font.AllUppercase
    font.letterSpacing: 0.6
  }

  Repeater {
    model: root.events
    Rectangle {
      id: item
      required property var modelData
      width: root.width
      height: line.implicitHeight + 2 * Style.spacing.sm
      radius: Style.space(8)
      color: Util.alpha(root.textColor, mouse.containsMouse ? 0.1 : 0.05)
      Behavior on color { ColorAnimation { duration: 120 } }
      Rectangle {
        x: Style.spacing.sm
        anchors.verticalCenter: parent.verticalCenter
        width: Style.space(3)
        height: parent.height - 2 * Style.spacing.sm
        radius: width / 2
        color: Commons.Color.accent
      }
      Column {
        id: line
        x: Style.spacing.sm + Style.space(9)
        width: parent.width - x - Style.spacing.sm
        anchors.verticalCenter: parent.verticalCenter
        Text {
          width: parent.width
          text: item.modelData.title
          elide: Text.ElideRight
          textFormat: Text.PlainText
          color: root.textColor
          font.family: Style.font.family
          font.pixelSize: Style.font.bodySmall
          font.bold: true
        }
        Text {
          width: parent.width
          text: root.when(item.modelData)
          elide: Text.ElideRight
          color: root.dimColor
          font.family: Style.font.family
          font.pixelSize: Style.font.caption
          font.features: { "tnum": 1 }
        }
      }
      MouseArea {
        id: mouse
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: root.openDay(item.modelData.on)
      }
    }
  }

  // Nothing coming up.
  Column {
    visible: root.events.length === 0
    width: root.width
    spacing: Style.spacing.xs
    Text {
      width: parent.width
      wrapMode: Text.WordWrap
      text: root.island.tr("home_nothing")
      color: root.textColor
      font.family: Style.font.family
      font.pixelSize: Style.font.bodySmall
    }
    IslandPill {
      label: root.island.tr("home_add")
      glyph: "󰐕"
      onClicked: root.openDay(root.today)
    }
  }
}
