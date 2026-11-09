import QtQuick
import qs.Commons
import qs.Commons as Commons

// The ring around the control that has the keyboard: drawn just outside
// its parent, in the accent, only while it has focus.
Rectangle {
  anchors.fill: parent
  anchors.margins: -Style.space(2)
  visible: parent.activeFocus
  color: "transparent"
  border.width: Style.space(2)
  border.color: Commons.Color.accent
  radius: Style.space(8)
}
