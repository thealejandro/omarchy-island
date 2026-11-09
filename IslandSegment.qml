import QtQuick
import qs.Commons
import qs.Commons as Commons
import "IslandModel.js" as IslandModel

// A row of choices with one picked, for enum settings. options is a list
// of { value, label }; values are compared as JSON so null works too.
// Tab reaches it, ← and → pick the neighbour.
Rectangle {
  id: root

  property var options: []
  property var value: null
  property string label: ""
  signal picked(var value)

  function step(delta) {
    var next = IslandModel.stepChoice(options.map(function(o) { return o.value }), value, delta)
    if (JSON.stringify(next) !== JSON.stringify(value)) picked(next)
  }

  activeFocusOnTab: visible && enabled
  Keys.onLeftPressed: step(-1)
  Keys.onRightPressed: step(1)
  Accessible.role: Accessible.PageTabList
  Accessible.name: label
  Accessible.focusable: true

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
        Accessible.role: Accessible.PageTab
        Accessible.name: modelData.label
        Accessible.checkable: true
        Accessible.checked: active
        Accessible.onPressAction: root.picked(chip.modelData.value)

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

  IslandFocusRing { radius: root.radius + Style.space(2) }
}
