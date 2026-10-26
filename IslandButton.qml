import QtQuick
import qs.Commons
import qs.Commons as Commons

// Round icon button. "filled" is the primary action (play), drawn as an
// accent disc; the rest are bare glyphs with a soft hover halo.
Item {
  id: root

  property string glyph: ""
  property bool filled: false
  property bool accent: false
  property int size: Style.space(30)
  property int glyphSize: Style.font.iconLarge
  signal clicked()

  implicitWidth: size
  implicitHeight: size

  Rectangle {
    anchors.fill: parent
    radius: width / 2
    color: root.filled ? Commons.Color.accent
      : Util.alpha(Commons.Color.bar.text, mouse.containsMouse ? 0.1 : 0)
    scale: mouse.pressed ? 0.9 : (mouse.containsMouse && root.filled ? 1.06 : 1)
    Behavior on color { ColorAnimation { duration: 120 } }
    Behavior on scale { NumberAnimation { duration: 120; easing.type: Easing.OutQuad } }
  }

  Glyph {
    anchors.centerIn: parent
    // A play triangle looks centered on its centroid, not its box.
    anchors.horizontalCenterOffset: root.glyph === "󰐊" ? Math.round(width * 0.1) : 0
    text: root.glyph
    color: root.filled ? Commons.Color.bar.background
      : (root.accent ? Commons.Color.accent : Commons.Color.bar.text)
    opacity: root.filled || mouse.containsMouse ? 1 : 0.75
    font.family: Style.font.family
    font.pixelSize: root.glyphSize
  }

  MouseArea {
    id: mouse
    anchors.fill: parent
    hoverEnabled: true
    cursorShape: Qt.PointingHandCursor
    onClicked: root.clicked()
  }
}
