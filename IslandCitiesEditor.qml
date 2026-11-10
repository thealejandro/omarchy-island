import QtQuick
import Quickshell.Io
import qs.Commons
import qs.Commons as Commons
import "IslandCities.js" as IslandCities

// Settings › World's city list: one row per city with buttons to move it up
// or down or remove it, and a row to add another (name + zone). The zone is
// picked from a searcher over the IANA zone list, fetched from
// `timedatectl list-timezones` once, the first time the searcher opens, with
// a short built-in list as fallback. Every change goes out through
// accepted(), which the settings page saves like any other setting; the
// buttons join the Tab walk and carry accessible names like any other
// island control.
Column {
  id: root

  required property var island
  property var cities: []
  signal accepted(var cities)

  readonly property color textColor: Commons.Color.bar.text
  readonly property color dimColor: Util.alpha(Commons.Color.bar.text, 0.72)

  spacing: Style.spacing.sm

  function tr(key, arg) { return root.island.tr(key, arg) }

  // The searcher's list. It starts as the fallback so the searcher always
  // answers, even if the fetch below never comes back; a successful run
  // replaces it with the real zones.
  property var zones: IslandCities.FALLBACK_ZONES
  property bool zonesRequested: false
  readonly property var matches: zoneField.editing
    ? IslandCities.filterZones(root.zones, zoneField.current, 6) : []

  Process {
    id: tzFetch
    command: ["timedatectl", "list-timezones"]
    running: false
    stdout: StdioCollector {
      onStreamFinished: {
        var list = IslandCities.parseZoneLines(text)
        root.zones = list.length > 0 ? list : IslandCities.FALLBACK_ZONES
      }
    }
    onExited: {
      if (root.zones.length === 0) root.zones = IslandCities.FALLBACK_ZONES
    }
  }

  function openSearcher() {
    if (root.zonesRequested) return
    root.zonesRequested = true
    tzFetch.running = true
  }

  function tryAdd() {
    var next = IslandCities.add(root.cities, nameField.current, zoneField.current)
    if (!next) return
    root.accepted(next)
    nameField.put("")
    zoneField.put("")
  }

  Repeater {
    model: root.cities

    Item {
      id: cityRow
      required property var modelData
      required property int index
      width: root.width
      height: Style.space(30)

      Column {
        anchors.left: parent.left
        anchors.right: buttons.left
        anchors.rightMargin: Style.spacing.md
        anchors.verticalCenter: parent.verticalCenter

        Text {
          width: parent.width
          elide: Text.ElideRight
          text: cityRow.modelData.name
          color: root.textColor
          font.family: Style.font.family
          font.pixelSize: Style.font.bodySmall
          font.bold: true
        }
        Text {
          width: parent.width
          elide: Text.ElideRight
          text: cityRow.modelData.zone
          color: root.dimColor
          font.family: Style.font.family
          font.pixelSize: Style.font.caption
        }
      }

      Row {
        id: buttons
        anchors.right: parent.right
        anchors.verticalCenter: parent.verticalCenter
        spacing: Style.spacing.xs

        IslandButton {
          glyph: "󰅃"
          label: root.tr("a11y_citiesUp", cityRow.modelData.name)
          size: Style.space(24)
          glyphSize: Style.font.bodySmall
          opacity: cityRow.index > 0 ? 1 : 0.35
          onClicked: root.accepted(IslandCities.move(root.cities, cityRow.index, -1))
        }
        IslandButton {
          glyph: "󰅀"
          label: root.tr("a11y_citiesDown", cityRow.modelData.name)
          size: Style.space(24)
          glyphSize: Style.font.bodySmall
          opacity: cityRow.index < root.cities.length - 1 ? 1 : 0.35
          onClicked: root.accepted(IslandCities.move(root.cities, cityRow.index, 1))
        }
        IslandButton {
          glyph: "󰅖"
          label: root.tr("a11y_citiesRemove", cityRow.modelData.name)
          size: Style.space(24)
          glyphSize: Style.font.bodySmall
          opacity: 0.75
          onClicked: root.accepted(IslandCities.remove(root.cities, cityRow.index))
        }
      }
    }
  }

  Row {
    id: addRow
    width: root.width
    spacing: Style.spacing.sm

    IslandField {
      id: nameField
      island: root.island
      label: root.tr("cities_name")
      placeholder: root.tr("cities_name")
      revertOnLeave: false
      width: Math.max(Style.space(96), root.width * 0.3)
      onAccepted: root.tryAdd()
    }
    IslandField {
      id: zoneField
      island: root.island
      label: root.tr("cities_zone")
      placeholder: root.tr("cities_zone")
      revertOnLeave: false
      width: Math.max(Style.space(140), root.width * 0.44)
      onAccepted: root.tryAdd()
      onEditingChanged: if (editing) root.openSearcher()
    }
    IslandButton {
      id: addButton
      anchors.verticalCenter: parent.verticalCenter
      glyph: "󰐕"
      label: root.tr("a11y_citiesAdd")
      size: Style.space(26)
      opacity: IslandCities.isValidZone(zoneField.current) ? 1 : 0.4
      onClicked: root.tryAdd()
    }
  }

  // The searcher's results, under the add row while its zone field has the
  // keyboard: the typed text filters the zone list, and a click fills the
  // field with the picked zone.
  Column {
    width: root.width
    visible: zoneField.editing
    spacing: Style.spacing.xs

    Repeater {
      model: root.matches

      Rectangle {
        required property var modelData
        readonly property bool picked: zoneField.current.trim() === modelData
        width: root.width
        height: Style.space(24)
        radius: Style.space(7)
        color: Util.alpha(root.textColor, zoneMouse.containsMouse ? 0.14 : (picked ? 0.1 : 0.05))
        Behavior on color { ColorAnimation { duration: 100 } }

        Text {
          anchors.fill: parent
          anchors.leftMargin: Style.spacing.lg
          anchors.rightMargin: Style.spacing.lg
          verticalAlignment: Text.AlignVCenter
          text: modelData
          color: root.textColor
          elide: Text.ElideRight
          font.family: Style.font.family
          font.pixelSize: Style.font.bodySmall
        }

        MouseArea {
          id: zoneMouse
          anchors.fill: parent
          hoverEnabled: true
          cursorShape: Qt.PointingHandCursor
          onClicked: zoneField.put(modelData)
        }
      }
    }

    Text {
      width: root.width
      visible: zoneField.editing && root.matches.length === 0
      text: root.tr("cities_noMatches")
      color: root.dimColor
      font.family: Style.font.family
      font.pixelSize: Style.font.caption
    }
  }
}
