import QtQuick
import qs.Commons
import qs.Commons as Commons

// Adding or changing one of the island's events: title, time (empty for
// all day), repeat and when to be reminded. `event` is null for a new one.
Column {
  id: root

  required property var island
  property var event: null
  property string date: ""
  signal done()

  readonly property color textColor: Commons.Color.bar.text
  readonly property color dimColor: Util.alpha(Commons.Color.bar.text, 0.72)
  property string repeat: event ? event.repeat : "none"
  property var remind: event ? event.remind : null
  property bool invalid: false

  function tr(key, arg) { return island.tr(key, arg) }

  function save() {
    var fields = { date: event ? event.date : date, time: timeField.current, title: titleField.current,
      repeat: repeat, remind: remind }
    var ok = event ? island.updateEvent(event.id, fields) : island.addEvent(fields)
    invalid = !ok
    if (ok) { island.wantsKeyboard = false; done() }
  }

  spacing: Style.spacing.md

  component Pill: Rectangle {
    id: pill
    property string label: ""
    property bool accent: false
    signal clicked()
    width: pillText.implicitWidth + 2 * Style.spacing.xl
    height: Style.space(26)
    radius: Style.space(8)
    color: accent ? Util.alpha(Commons.Color.accent, pillMouse.containsMouse ? 0.32 : 0.22)
      : Util.alpha(root.textColor, pillMouse.containsMouse ? 0.13 : 0.07)
    scale: pillMouse.pressed ? 0.95 : 1
    Behavior on scale { NumberAnimation { duration: 140 } }
    Text {
      id: pillText
      anchors.centerIn: parent
      text: pill.label
      color: pill.accent ? Commons.Color.accent : root.textColor
      font.family: Style.font.family
      font.pixelSize: Style.font.caption
      font.bold: true
    }
    MouseArea {
      id: pillMouse
      anchors.fill: parent
      hoverEnabled: true
      cursorShape: Qt.PointingHandCursor
      onClicked: pill.clicked()
    }
  }

  Row {
    width: parent.width
    spacing: Style.spacing.md
    IslandField {
      id: titleField
      island: root.island
      width: parent.width - timeField.width - parent.spacing
      text: root.event ? root.event.title : ""
      placeholder: root.tr("ev_title")
      onAccepted: timeField.begin()
    }
    IslandField {
      id: timeField
      island: root.island
      width: Style.space(76)
      text: root.event ? root.event.time : ""
      placeholder: "HH:MM"
      onAccepted: root.save()
    }
  }

  Column {
    spacing: Style.spacing.xs
    Text {
      text: root.tr("ev_repeat")
      color: root.dimColor
      font.family: Style.font.family
      font.pixelSize: Style.font.caption
    }
    IslandSegment {
      options: ["none", "weekly", "monthly", "yearly"].map(function(v) { return { value: v, label: root.tr("rep_" + v) } })
      value: root.repeat
      onPicked: (v) => root.repeat = v
    }
  }

  Column {
    spacing: Style.spacing.xs
    Text {
      text: root.tr("ev_remind")
      color: root.dimColor
      font.family: Style.font.family
      font.pixelSize: Style.font.caption
    }
    IslandSegment {
      options: [{ value: null, label: root.tr("opt_auto") }, { value: 0, label: "0" },
        { value: 5, label: "5" }, { value: 10, label: "10" }, { value: 30, label: "30" }, { value: 60, label: "60" }]
      value: root.remind
      onPicked: (v) => root.remind = v
    }
  }

  Row {
    spacing: Style.spacing.sm
    Pill { label: root.tr("ev_save"); accent: true; onClicked: root.save() }
    Pill { label: root.tr("ev_cancel"); onClicked: { root.island.wantsKeyboard = false; root.done() } }
    Pill {
      visible: !!root.event
      label: root.tr("ev_delete")
      onClicked: { root.island.removeEvent(root.event.id); root.island.wantsKeyboard = false; root.done() }
    }
    Text {
      anchors.verticalCenter: parent.verticalCenter
      visible: root.invalid
      text: root.tr("ev_invalid")
      color: Commons.Color.urgent
      font.family: Style.font.family
      font.pixelSize: Style.font.caption
    }
  }

  Component.onCompleted: if (!event) titleField.begin()
}
