import QtQuick
import qs.Commons
import qs.Commons as Commons
import "IslandSettings.js" as IslandSettings
import "IslandModel.js" as IslandModel
import "IslandIcons.js" as IslandIcons

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
    if (key === "hoverDelay" || key === "peekDuration" || key === "notifyPeekDuration" || key === "agentPeekDuration" || key === "notifyKeyboardIdle") return " ms"
    if (key === "notifyHistoryLimit") return ""
    if (key === "reminderLead" || key === "icsRefresh" || key === "agentStale") return " min"
    return " px"
  }

  function save(key, value) {
    if (key === "osd") island.setOsd(value)
    else if (key === "notifications") island.setNotifications(value)
    else island.saveSetting(key, value)
  }

  component Pill: Rectangle {
    id: pill
    property string label: ""
    signal clicked()
    activeFocusOnTab: visible
    Keys.onSpacePressed: pill.clicked()
    Keys.onReturnPressed: pill.clicked()
    Keys.onEnterPressed: pill.clicked()
    Accessible.role: Accessible.Button
    Accessible.name: label
    Accessible.onPressAction: pill.clicked()
    IslandFocusRing { radius: pill.radius + Style.space(2) }
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

  // Sections. Wraps onto a second line on a narrow island. One Tab stop,
  // on the current section; ← and → change it.
  Flow {
    width: parent.width
    spacing: Style.spacing.sm
    Accessible.role: Accessible.PageTabList
    Accessible.name: root.tr("a11y_sections")
    function go(delta) {
      var next = IslandModel.stepChoice(IslandSettings.SECTIONS, root.section, delta)
      if (next === root.section) return
      root.section = next
      var chip = sectionChips.itemAt(IslandSettings.SECTIONS.indexOf(next))
      if (chip) chip.forceActiveFocus()
    }
    Repeater {
      id: sectionChips
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
        activeFocusOnTab: active
        Keys.onLeftPressed: chip.parent.go(-1)
        Keys.onRightPressed: chip.parent.go(1)
        Accessible.role: Accessible.PageTab
        Accessible.name: root.tr("sec_" + modelData)
        Accessible.checkable: true
        Accessible.checked: active
        Accessible.onPressAction: root.section = chip.modelData
        IslandFocusRing { radius: chip.radius + Style.space(2) }
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
        // A wide control (the six-way idle look) would squeeze the label to a
        // couple of letters: then the label goes on top and the control below.
        readonly property bool stacked: control.width > row.width - Style.space(150)
        width: rows.width
        height: stacked ? labels.height + Style.spacing.sm + control.height
                        : Math.max(labels.height, control.height)

        Column {
          id: labels
          x: 0
          y: row.stacked ? 0 : (row.height - height) / 2
          width: row.stacked ? row.width : Math.max(0, row.width - control.width - Style.spacing.xl)
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
          x: row.stacked ? 0 : row.width - width
          y: row.stacked ? labels.height + Style.spacing.sm : (row.height - height) / 2
          sourceComponent: {
            if (row.key === "idleIcon") return iconPickerComp
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
            label: root.tr("set_" + row.key)
            checked: row.value === true
            onToggled: (v) => root.save(row.key, v)
          }
        }
        Component {
          id: stepperComp
          IslandStepper {
            label: root.tr("set_" + row.key)
            value: row.value
            from: row.modelData.min
            to: row.modelData.max
            step: row.modelData.step
            suffix: root.suffixFor(row.key)
            onStepped: (v) => root.save(row.key, v)
          }
        }
        // The icon in the notch: ‹ name › through the built-ins and the user's
        // files, with a live preview of what the notch will show.
        Component {
          id: iconPickerComp
          Row {
            id: picker
            readonly property var names: IslandIcons.pickerList(root.island.iconFiles, String(row.value || ""))
            readonly property int index: Math.max(0, names.indexOf(String(row.value || "")))
            readonly property string shown: names[index] || ""
            spacing: Style.spacing.md
            Rectangle {
              anchors.verticalCenter: parent.verticalCenter
              width: Style.space(48)
              height: Style.space(22)
              radius: Style.space(7)
              color: Util.alpha(root.textColor, 0.07)
              IdleIcon {
                anchors.centerIn: parent
                builtin: root.island.iconBuiltin
                source: root.island.iconUrl
                tinted: root.island.iconTinted
                tint: Commons.Color.accent
                detailTint: Commons.Color.foreground
                along: Style.space(36)
                across: Style.space(12)
              }
            }
            IslandStepper {
              anchors.verticalCenter: parent.verticalCenter
              label: root.tr("set_idleIcon")
              text: picker.shown.length > 22 ? picker.shown.slice(0, 21) + "…" : picker.shown
              value: picker.index
              from: 0
              to: Math.max(0, picker.names.length - 1)
              leftGlyph: "‹"
              rightGlyph: "›"
              leftLabel: "‹"
              rightLabel: "›"
              wrap: true
              onStepped: (v) => root.save(row.key, picker.names[v])
            }
          }
        }
        Component {
          id: segmentComp
          IslandSegment {
            label: root.tr("set_" + row.key)
            options: root.optionsFor(row.modelData)
            value: row.value
            onPicked: (v) => root.save(row.key, v)
          }
        }
        Component {
          id: fieldComp
          IslandField {
            island: root.island
            label: root.tr("set_" + row.key)
            revertOnLeave: true
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
    Pill {
      label: root.tr(root.island.notificationsEnabled ? "act_notifyOff" : "act_notifyOn")
      onClicked: root.island.setNotifications(root.island.notificationsEnabled ? "off" : "island")
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
        : ((root.section === "system" || root.section === "notify") && root.island.notifyFailed ? root.tr("note_notifyFailed")
        : ((root.section === "system" || root.section === "notify") && root.island.notificationsStandby ? root.tr("note_notifyStandby")
        : (root.section === "system" ? root.tr("note_reset") : ""))))
    color: root.dimColor
    font.family: Style.font.family
    font.pixelSize: Style.font.caption
  }

  // Who made it, small and quiet, at the very end.
  Text {
    width: parent.width
    topPadding: Style.spacing.sm
    horizontalAlignment: Text.AlignRight
    text: root.tr("credit", "thealejandro.island" + (root.island.version ? " v" + root.island.version : ""))
    color: Util.alpha(Commons.Color.bar.text, 0.45)
    font.family: Style.font.family
    font.pixelSize: Style.font.caption
  }
}
