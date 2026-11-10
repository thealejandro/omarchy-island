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
  property real maxWidth: 10000
  property int revision: 0
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

  readonly property real naturalWidth: {
    revision
    var total = 2 * Style.space(2)
    for (var i = 0; i < choices.count; i++) {
      var chip = choices.itemAt(i)
      if (chip) total += chip.implicitWidth + (i > 0 ? row.spacing : 0)
    }
    return total
  }
  implicitWidth: Math.min(maxWidth, naturalWidth)
  implicitHeight: row.implicitHeight + 2 * Style.space(2)
  radius: Style.space(8)
  color: Util.alpha(Commons.Color.bar.text, 0.07)

  Flow {
    id: row
    x: Style.space(2)
    y: Style.space(2)
    width: Math.max(0, root.width - 2 * x)
    spacing: Style.space(2)

    Repeater {
      id: choices
      onItemAdded: root.revision++
      onItemRemoved: root.revision++
      model: root.options

      Rectangle {
        id: chip
        required property var modelData
        readonly property bool active: JSON.stringify(modelData.value) === JSON.stringify(root.value)
        implicitWidth: label.implicitWidth + 2 * Style.spacing.lg
        width: Math.min(implicitWidth, row.width)
        height: Style.space(20)
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
          width: Math.max(0, chip.width - 2 * Style.spacing.lg)
          horizontalAlignment: Text.AlignHCenter
          elide: Text.ElideRight
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
