import QtQuick
import qs.Commons
import qs.Commons as Commons

Text {
  id: root

  property string glyph: ""
  property bool accent: false
  signal clicked()

  text: glyph
  color: accent ? Commons.Color.accent : Commons.Color.bar.text
  opacity: mouse.containsMouse ? 1 : 0.8
  scale: mouse.pressed ? 0.88 : 1
  font.family: Style.font.family
  font.pixelSize: Style.font.iconLarge

  Behavior on scale { NumberAnimation { duration: 90 } }

  MouseArea {
    id: mouse
    anchors.fill: parent
    anchors.margins: -Style.space(4)
    hoverEnabled: true
    cursorShape: Qt.PointingHandCursor
    onClicked: root.clicked()
  }
}
