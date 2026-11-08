import QtQuick
import qs.Commons
import qs.Commons as Commons

// A row of choices with one picked, for enum settings. options is a list
// of { value, label }; values are compared as JSON so null works too.
Rectangle {
  id: root

  property var options: []
  property var value: null
  signal picked(var value)

  implicitWidth: row.implicitWidth + 2 * Style.space(2)
  implicitHeight: Style.space(24)
  radius: Style.space(8)
  color: Util.alpha(Commons.Color.bar.text, 0.07)

  Row {
    id: row
    anchors.centerIn: parent
    spacing: Style.space(2)

    Repeater {
      model: root.options

      Rectangle {
        id: chip
        required property var modelData
        readonly property bool active: JSON.stringify(modelData.value) === JSON.stringify(root.value)
        width: label.implicitWidth + 2 * Style.spacing.lg
        height: root.height - 2 * Style.space(2)
        radius: Style.space(6)
        color: active ? Util.alpha(Commons.Color.accent, 0.22)
          : Util.alpha(Commons.Color.bar.text, mouse.containsMouse ? 0.08 : 0)
        Behavior on color { ColorAnimation { duration: 140 } }

        Text {
          id: label
          anchors.centerIn: parent
          text: chip.modelData.label
          color: chip.active ? Commons.Color.accent : Commons.Color.bar.text
          opacity: chip.active || mouse.containsMouse ? 1 : 0.78
          font.family: Style.font.family
          font.pixelSize: Style.font.caption
          font.bold: chip.active
        }
        MouseArea {
          id: mouse
          anchors.fill: parent
          hoverEnabled: true
          cursorShape: Qt.PointingHandCursor
          onClicked: root.picked(chip.modelData.value)
        }
      }
    }
  }
}
