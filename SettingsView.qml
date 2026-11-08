import QtQuick
import qs.Commons
import qs.Commons as Commons
import "IslandSettings.js" as IslandSettings

// The settings page, drawn from IslandSettings' schema, one section at a
// time. Every change is written to the island's entry in shell.json right
// away, the same entry `omarchy bar set thealejandro.island …` edits.
Column {
  id: root

  required property var island
  property string section: "general"

  readonly property color textColor: Commons.Color.bar.text
  readonly property color dimColor: Util.alpha(Commons.Color.bar.text, 0.72)

  spacing: Style.spacing.lg

  function tr(key, arg) { return island.tr(key, arg) }
  function has(key) { return tr(key) !== key }

  function optionsFor(s) {
    if (s.type === "tristate")
      return [{ value: null, label: tr("opt_auto") }, { value: false, label: tr("opt_h12") }, { value: true, label: tr("opt_h24") }]
    if (s.type === "weekday")
      return [{ value: null, label: tr("opt_auto") }, { value: 1, label: tr("opt_mon") },
        { value: 6, label: tr("opt_sat") }, { value: 7, label: tr("opt_sun") }]
    return s.options.map(function(o) {
      if (s.key === "language") return { value: o, label: o === "" ? tr("opt_auto") : (o === "es" ? "Español" : "English") }
      return { value: o, label: tr("opt_" + o) }
    })
  }

  function suffixFor(key) {
    if (key === "hoverDelay" || key === "peekDuration") return " ms"
    if (key === "reminderLead") return " min"
    return " px"
  }

  function save(key, value) {
    if (key === "osd") island.setOsd(value)
    else island.saveSetting(key, value)
  }

  component Pill: Rectangle {
    id: pill
    property string label: ""
    signal clicked()
    width: pillText.implicitWidth + 2 * Style.spacing.xl
    height: Style.space(26)
    radius: Style.space(8)
    color: Util.alpha(root.textColor, pillMouse.containsMouse ? 0.13 : 0.07)
    scale: pillMouse.pressed ? 0.95 : 1
    Behavior on color { ColorAnimation { duration: 120 } }
    Behavior on scale { NumberAnimation { duration: 140 } }
    Text {
      id: pillText
      anchors.centerIn: parent
      text: pill.label
      color: root.textColor
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

  // Sections. Wraps onto a second line on a narrow island.
  Flow {
    width: parent.width
    spacing: Style.spacing.sm
    Repeater {
      model: IslandSettings.SECTIONS
      Rectangle {
        id: chip
        required property string modelData
        readonly property bool active: root.section === modelData
        width: chipText.implicitWidth + 2 * Style.spacing.lg
        height: Style.space(22)
        radius: Style.space(7)
        color: active ? Util.alpha(Commons.Color.accent, 0.2)
          : Util.alpha(root.textColor, chipMouse.containsMouse ? 0.1 : 0.05)
        Behavior on color { ColorAnimation { duration: 140 } }
        Text {
          id: chipText
          anchors.centerIn: parent
          text: root.tr("sec_" + chip.modelData)
          color: chip.active ? Commons.Color.accent : root.textColor
          font.family: Style.font.family
          font.pixelSize: Style.font.caption
          font.bold: chip.active
        }
        MouseArea {
          id: chipMouse
          anchors.fill: parent
          hoverEnabled: true
          cursorShape: Qt.PointingHandCursor
          onClicked: root.section = chip.modelData
        }
      }
    }
  }

  Column {
    id: rows
    width: parent.width
    spacing: Style.spacing.md

    Repeater {
      model: IslandSettings.inSection(root.section)

      Item {
        id: row
        required property var modelData
        readonly property string key: modelData.key
        readonly property var value: root.island.cfg[key]
        readonly property string hintKey: "hint_" + key
        width: rows.width
        height: Math.max(labels.height, control.height)

        Column {
          id: labels
          anchors.left: parent.left
          anchors.verticalCenter: parent.verticalCenter
          width: Math.max(0, row.width - control.width - Style.spacing.xl)
          Text {
            width: parent.width
            text: root.tr("set_" + row.key)
            color: root.textColor
            elide: Text.ElideRight
            font.family: Style.font.family
            font.pixelSize: Style.font.bodySmall
          }
          Text {
            width: parent.width
            visible: root.has(row.hintKey)
            text: visible ? root.tr(row.hintKey) : ""
            color: root.dimColor
            elide: Text.ElideRight
            font.family: Style.font.family
            font.pixelSize: Style.font.caption
          }
        }

        Loader {
          id: control
          anchors.right: parent.right
          anchors.verticalCenter: parent.verticalCenter
          sourceComponent: {
            switch (row.modelData.type) {
              case "bool": return toggleComp
              case "int": return stepperComp
              case "string": case "cities": return fieldComp
              default: return segmentComp
            }
          }
        }

        Component {
          id: toggleComp
          IslandToggle {
            checked: row.value === true
            onToggled: (v) => root.save(row.key, v)
          }
        }
        Component {
          id: stepperComp
          IslandStepper {
            value: row.value
            from: row.modelData.min
            to: row.modelData.max
            step: row.modelData.step
            suffix: root.suffixFor(row.key)
            onStepped: (v) => root.save(row.key, v)
          }
        }
        Component {
          id: segmentComp
          IslandSegment {
            options: root.optionsFor(row.modelData)
            value: row.value
            onPicked: (v) => root.save(row.key, v)
          }
        }
        Component {
          id: fieldComp
          IslandField {
            island: root.island
            width: Math.min(Style.space(240), rows.width * 0.5)
            text: row.modelData.type === "cities" ? IslandSettings.citiesText(row.value) : String(row.value || "")
            placeholder: row.modelData.type === "cities" ? "Tokyo|Asia/Tokyo" : ""
            onAccepted: (t) => root.save(row.key, row.modelData.type === "cities" ? IslandSettings.parseCities(t) : t)
          }
        }
      }
    }
  }

  // Things that need Omarchy to change too, and the way back.
  Flow {
    width: parent.width
    spacing: Style.spacing.sm
    visible: root.section === "system"
    Pill {
      label: root.tr(root.island.osdEnabled ? "act_osdOff" : "act_osdOn")
      onClicked: root.island.setOsd(!root.island.osdEnabled)
    }
    Pill { label: root.tr("act_place"); onClicked: root.island.placeByClock() }
    Pill { label: root.tr("act_openConfig"); onClicked: root.island.openConfig() }
    Pill { label: root.tr("act_reset"); onClicked: root.island.resetSettings() }
  }

  Text {
    width: parent.width
    visible: text !== ""
    wrapMode: Text.WordWrap
    text: !root.island.hasEntry ? root.tr("note_noEntry")
      : (!root.island.omarchyTested ? root.tr("note_version", root.island.omarchyVersion)
        : (root.section === "system" ? root.tr("note_reset") : ""))
    color: root.dimColor
    font.family: Style.font.family
    font.pixelSize: Style.font.caption
  }
}
