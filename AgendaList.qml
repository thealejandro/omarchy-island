import QtQuick
import qs.Commons
import qs.Commons as Commons
import "AgendaModel.js" as AgendaModel

// One day of the agenda: its events (the island's own and the external
// command's), Omarchy's reminders when it's today, and what's coming in the
// next week. Events can be added, changed and handed to Omarchy's
// reminders from here.
Column {
  id: root

  required property var island
  property string day: AgendaModel.isoDate(new Date())

  readonly property color textColor: Commons.Color.bar.text
  readonly property color dimColor: Util.alpha(Commons.Color.bar.text, 0.72)
  readonly property bool isToday: day === AgendaModel.isoDate(new Date())
  readonly property var dayEvents: AgendaModel.eventsOn(island.allEvents, day)
  readonly property var next: {
    var d = AgendaModel.parseDate(day) || new Date()
    return AgendaModel.upcoming(island.allEvents, new Date(d.getFullYear(), d.getMonth(), d.getDate() + 1), 7)
  }
  // null: not editing; {} : a new event; an event: changing it.
  property var editing: null
  property string notice: ""

  function tr(key, arg) { return island.tr(key, arg) }
  function dayTitle(iso) {
    var d = AgendaModel.parseDate(iso)
    if (!d) return ""
    if (iso === AgendaModel.isoDate(new Date())) return tr("today")
    var t = island.locale.toString(d, "dddd d MMMM")
    return t.charAt(0).toUpperCase() + t.slice(1)
  }
  function shortDay(iso) {
    var d = AgendaModel.parseDate(iso)
    return d ? island.locale.toString(d, "ddd d") : iso
  }

  onDayChanged: { editing = null; notice = "" }

  spacing: Style.spacing.md

  Item {
    width: parent.width
    height: Style.space(24)
    Text {
      anchors.left: parent.left
      anchors.right: addButton.left
      anchors.verticalCenter: parent.verticalCenter
      elide: Text.ElideRight
      text: root.dayTitle(root.day)
      color: root.textColor
      font.family: Style.font.family
      font.pixelSize: Style.font.title
      font.bold: true
    }
    IslandButton {
      id: addButton
      anchors.right: parent.right
      anchors.verticalCenter: parent.verticalCenter
      visible: root.editing === null
      glyph: "󰐕"
      label: root.tr("a11y_add")
      size: Style.space(24)
      glyphSize: Style.font.icon
      onClicked: root.editing = ({})
    }
  }

  Loader {
    width: parent.width
    active: root.editing !== null
    visible: active
    sourceComponent: EventEditor {
      island: root.island
      event: root.editing && root.editing.id ? root.editing : null
      date: root.day
      onDone: root.editing = null
    }
  }

  Column {
    width: parent.width
    spacing: Style.spacing.xs
    visible: root.editing === null

    Repeater {
      model: root.dayEvents
      Rectangle {
        id: row
        required property var modelData
        readonly property bool local: modelData.source === "local"
        readonly property int minutes: AgendaModel.minutesUntil(modelData, root.day, new Date()) || 0
        width: parent.width
        height: Style.space(30)
        radius: Style.space(8)
        color: Util.alpha(root.textColor, rowMouse.containsMouse ? 0.08 : 0.04)

        MouseArea {
          id: rowMouse
          anchors.fill: parent
          hoverEnabled: true
          cursorShape: row.local ? Qt.PointingHandCursor : Qt.ArrowCursor
          onClicked: if (row.local) root.editing = root.island.localEvents.filter(function(e) { return e.id === row.modelData.id })[0] || null
        }
        Text {
          id: when
          anchors.left: parent.left
          anchors.leftMargin: Style.spacing.lg
          anchors.verticalCenter: parent.verticalCenter
          width: Math.max(Style.space(52), implicitWidth + Style.spacing.lg)
          text: row.modelData.time || root.tr("allDay")
          color: Commons.Color.accent
          font.family: Style.font.family
          font.pixelSize: Style.font.caption
          font.bold: true
          font.features: { "tnum": 1 }
        }
        Text {
          anchors.left: when.right
          anchors.right: icons.left
          anchors.rightMargin: Style.spacing.sm
          anchors.verticalCenter: parent.verticalCenter
          elide: Text.ElideRight
          text: row.modelData.title
          color: root.textColor
          font.family: Style.font.family
          font.pixelSize: Style.font.bodySmall
        }
        Row {
          id: icons
          anchors.right: parent.right
          anchors.rightMargin: Style.spacing.sm
          anchors.verticalCenter: parent.verticalCenter
          spacing: Style.spacing.xs
          Glyph {
            anchors.verticalCenter: parent.verticalCenter
            visible: row.modelData.repeat !== "none"
            text: "󰑖"
            color: root.dimColor
            font.family: Style.font.family
            font.pixelSize: Style.font.caption
          }
          Glyph {
            anchors.verticalCenter: parent.verticalCenter
            visible: !row.local
            text: "󰌷"
            color: root.dimColor
            font.family: Style.font.family
            font.pixelSize: Style.font.caption
          }
          // Hand it to Omarchy's reminders (today, still ahead).
          IslandButton {
            visible: root.isToday && row.modelData.time !== "" && row.minutes > 0
            glyph: "󰂚"
            label: root.tr("a11y_remind")
            size: Style.space(22)
            glyphSize: Style.font.caption + Style.space(2)
            onClicked: root.notice = root.island.remindViaOmarchy(row.modelData, root.day)
              ? root.tr("ev_reminded") : root.tr("ev_tooLate")
          }
        }
      }
    }

    Text {
      visible: root.dayEvents.length === 0
      text: root.tr("ev_none")
      color: root.dimColor
      font.family: Style.font.family
      font.pixelSize: Style.font.caption
    }
    Text {
      visible: root.notice !== ""
      text: root.notice
      color: Commons.Color.accent
      font.family: Style.font.family
      font.pixelSize: Style.font.caption
    }
  }

  // Omarchy's own reminders, on today.
  Column {
    width: parent.width
    spacing: Style.spacing.xs
    visible: root.editing === null && root.isToday && root.island.omarchyReminders.length > 0
    Text {
      text: root.tr("ev_omarchyReminders")
      color: root.dimColor
      font.family: Style.font.family
      font.pixelSize: Style.font.caption
      font.bold: true
      font.capitalization: Font.AllUppercase
    }
    Repeater {
      model: root.island.omarchyReminders
      Text {
        required property var modelData
        width: parent.width
        elide: Text.ElideRight
        text: "󰂚  " + (modelData.at ? modelData.at + "  " : "") + (modelData.title || root.tr("reminders"))
        color: root.textColor
        font.family: Style.font.family
        font.pixelSize: Style.font.caption
      }
    }
  }

  Column {
    width: parent.width
    spacing: Style.spacing.xs
    visible: root.editing === null && root.next.length > 0
    Text {
      text: root.tr("ev_next")
      color: root.dimColor
      font.family: Style.font.family
      font.pixelSize: Style.font.caption
      font.bold: true
      font.capitalization: Font.AllUppercase
    }
    Repeater {
      model: root.next
      Text {
        required property var modelData
        width: parent.width
        elide: Text.ElideRight
        text: root.shortDay(modelData.date) + "  ·  " + modelData.events.map(function(e) {
          return (e.time ? e.time + " " : "") + e.title }).join(", ")
        color: root.textColor
        font.family: Style.font.family
        font.pixelSize: Style.font.caption
        MouseArea {
          anchors.fill: parent
          cursorShape: Qt.PointingHandCursor
          onClicked: root.island.calendarDay = parent.modelData.date
        }
      }
    }
  }
}
