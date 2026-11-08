import QtQuick
import qs.Commons
import qs.Commons as Commons

// − value + for whole numbers; the scroll wheel steps too.
Row {
  id: root

  property int value: 0
  property int from: 0
  property int to: 100
  property int step: 1
  property string suffix: ""
  signal stepped(int value)

  spacing: Style.spacing.sm

  function bump(delta) {
    var next = Math.max(from, Math.min(to, value + delta * step))
    if (next !== value) stepped(next)
  }

  IslandButton {
    anchors.verticalCenter: parent.verticalCenter
    glyph: "󰍴"
    size: Style.space(22)
    glyphSize: Style.font.bodySmall
    opacity: root.value > root.from ? 1 : 0.35
    onClicked: root.bump(-1)
  }
  Text {
    anchors.verticalCenter: parent.verticalCenter
    width: Math.max(implicitWidth, Style.space(58))
    horizontalAlignment: Text.AlignHCenter
    text: root.value + root.suffix
    color: Commons.Color.bar.text
    font.family: Style.font.family
    font.pixelSize: Style.font.bodySmall
    font.bold: true
    font.features: { "tnum": 1 }
  }
  IslandButton {
    anchors.verticalCenter: parent.verticalCenter
    glyph: "󰐕"
    size: Style.space(22)
    glyphSize: Style.font.bodySmall
    opacity: root.value < root.to ? 1 : 0.35
    onClicked: root.bump(1)
  }

  WheelHandler {
    acceptedDevices: PointerDevice.Mouse | PointerDevice.TouchPad
    onWheel: (event) => root.bump(event.angleDelta.y > 0 ? 1 : -1)
  }
}
