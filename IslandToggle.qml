import QtQuick
import qs.Commons
import qs.Commons as Commons

// A small on/off switch drawn like the dock's buttons: accent when on.
// Tab reaches it, Space or Enter flips it; "label" names it.
Item {
  id: root

  property bool checked: false
  property string label: ""
  signal toggled(bool value)

  implicitWidth: Style.space(36)
  implicitHeight: Style.space(20)
  activeFocusOnTab: visible && enabled
  Keys.onSpacePressed: root.toggled(!root.checked)
  Keys.onReturnPressed: root.toggled(!root.checked)
  Keys.onEnterPressed: root.toggled(!root.checked)
  Accessible.role: Accessible.CheckBox
  Accessible.name: label
  Accessible.checkable: true
  Accessible.checked: checked
  Accessible.onToggleAction: root.toggled(!root.checked)
  Accessible.onPressAction: root.toggled(!root.checked)

  Rectangle {
    anchors.fill: parent
    radius: height / 2
    color: root.checked ? Util.alpha(Commons.Color.accent, mouse.containsMouse ? 0.95 : 0.85)
      : Util.alpha(Commons.Color.bar.text, mouse.containsMouse ? 0.22 : 0.16)
    Behavior on color { ColorAnimation { duration: 160 } }

    Rectangle {
      width: parent.height - Style.space(6)
      height: width
      radius: width / 2
      anchors.verticalCenter: parent.verticalCenter
      x: root.checked ? parent.width - width - Style.space(3) : Style.space(3)
      color: root.checked ? Commons.Color.bar.background : Commons.Color.bar.text
      opacity: root.checked ? 1 : 0.7
      Behavior on x { NumberAnimation { duration: 200; easing.type: Easing.OutBack; easing.overshoot: 1.6 } }
    }
  }

  IslandFocusRing { radius: height / 2 + Style.space(2) }

  MouseArea {
    id: mouse
    anchors.fill: parent
    hoverEnabled: true
    cursorShape: Qt.PointingHandCursor
    onClicked: root.toggled(!root.checked)
  }
}
