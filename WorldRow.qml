import QtQuick
import qs.Commons
import qs.Commons as Commons

// The home page's world clock: one compact chip per city. A click opens
// Omarchy's elsewhen panel when that widget is on the bar, or the island's
// own world page otherwise.
Flow {
  id: root

  required property var island

  readonly property color textColor: Commons.Color.bar.text
  readonly property color dimColor: Util.alpha(Commons.Color.bar.text, 0.72)

  // Cities on the same time as here add nothing at a glance; the world
  // page still lists them.
  readonly property var rows: island.worldRows.filter(function(r) { return r.diffMinutes !== 0 })

  spacing: Style.spacing.sm
  visible: island.cfg.showWorld && rows.length > 0

  function open() {
    if (island.elsewhenOnBar) island.runAction(["omarchy-shell", "omarchy.elsewhen", "toggle"])
    else island.page = "world"
  }

  Repeater {
    model: root.rows

    Rectangle {
      id: chip
      required property var modelData
      width: chipRow.implicitWidth + 2 * Style.spacing.lg
      height: Style.space(24)
      radius: Style.space(8)
      color: Util.alpha(root.textColor, mouse.containsMouse ? 0.12 : 0.06)
      Behavior on color { ColorAnimation { duration: 120 } }

      Row {
        id: chipRow
        anchors.centerIn: parent
        spacing: Style.spacing.sm
        Glyph {
          anchors.verticalCenter: parent.verticalCenter
          text: chip.modelData.night ? "󰖔" : "󰖙"
          color: root.dimColor
          font.family: Style.font.family
          font.pixelSize: Style.font.caption
        }
        Text {
          anchors.verticalCenter: parent.verticalCenter
          text: chip.modelData.name
          color: root.dimColor
          font.family: Style.font.family
          font.pixelSize: Style.font.caption
        }
        Text {
          anchors.verticalCenter: parent.verticalCenter
          text: chip.modelData.time + (chip.modelData.meridiem ? " " + chip.modelData.meridiem : "")
            + (chip.modelData.dayDelta > 0 ? " +1" : (chip.modelData.dayDelta < 0 ? " -1" : ""))
          color: root.textColor
          font.family: Style.font.family
          font.pixelSize: Style.font.caption
          font.bold: true
          font.features: { "tnum": 1 }
        }
      }

      MouseArea {
        id: mouse
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: root.open()
      }
    }
  }
}
