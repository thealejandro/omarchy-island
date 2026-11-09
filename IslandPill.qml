import QtQuick
import qs.Commons
import qs.Commons as Commons

// A small text button: an optional glyph and a label. "on" lights it with
// the accent, as the toggles do. Tab reaches it, Space or Enter presses it.
Rectangle {
  id: pill

  property string label: ""
  property string glyph: ""
  property bool on: false
  property bool compact: false
  signal clicked()

  readonly property color textColor: on ? Commons.Color.accent : Commons.Color.bar.text

  implicitWidth: row.implicitWidth + 2 * (compact ? Style.spacing.lg : Style.spacing.xl)
  implicitHeight: Style.space(compact ? 22 : 26)
  activeFocusOnTab: visible && enabled
  Keys.onSpacePressed: pill.clicked()
  Keys.onReturnPressed: pill.clicked()
  Keys.onEnterPressed: pill.clicked()
  Accessible.role: Accessible.Button
  Accessible.name: label
  Accessible.onPressAction: pill.clicked()
  radius: Style.space(compact ? 7 : 8)
  color: on ? Util.alpha(Commons.Color.accent, mouse.containsMouse ? 0.3 : 0.2)
    : Util.alpha(Commons.Color.bar.text, mouse.containsMouse ? 0.13 : 0.07)
  scale: mouse.pressed ? 0.95 : 1
  Behavior on color { ColorAnimation { duration: 120 } }
  Behavior on scale { NumberAnimation { duration: 140 } }

  Row {
    id: row
    anchors.centerIn: parent
    spacing: Style.spacing.sm
    Glyph {
      visible: pill.glyph !== ""
      anchors.verticalCenter: parent.verticalCenter
      text: pill.glyph
      color: pill.textColor
      font.family: Style.font.family
      font.pixelSize: Style.font.bodySmall
    }
    Text {
      visible: pill.label !== ""
      anchors.verticalCenter: parent.verticalCenter
      text: pill.label
      color: pill.textColor
      font.family: Style.font.family
      font.pixelSize: Style.font.caption
      font.bold: true
    }
  }

  IslandFocusRing { radius: pill.radius + Style.space(2) }

  MouseArea {
    id: mouse
    anchors.fill: parent
    hoverEnabled: true
    cursorShape: Qt.PointingHandCursor
    onClicked: pill.clicked()
  }
}
