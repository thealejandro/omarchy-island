import QtQuick
import qs.Commons
import qs.Commons as Commons

// − value + for whole numbers (or, with `text`, any label between two arrows); the scroll wheel steps too. Tab reaches it
// as one stop, ← and → step it.
Item {
  id: root

  property int value: 0
  property int from: 0
  property int to: 100
  property int step: 1
  property string suffix: ""
  property string label: ""
  // Replaces the middle label; the owner then handles stepped(), which
  // carries the clamped value the arrows would have set.
  property string text: ""
  property string leftGlyph: "󰍴"
  property string rightGlyph: "󰐕"
  property string leftLabel: "−"
  property string rightLabel: "+"
  signal stepped(int value)

  implicitWidth: row.implicitWidth
  implicitHeight: row.implicitHeight
  activeFocusOnTab: visible && enabled
  Keys.onLeftPressed: bump(-1)
  Keys.onRightPressed: bump(1)
  Accessible.role: Accessible.SpinBox
  Accessible.name: label
  Accessible.description: text !== "" ? text : value + suffix
  Accessible.focusable: true

  function bump(delta) {
    var next = Math.max(from, Math.min(to, value + delta * step))
    if (next !== value) stepped(next)
  }

  Row {
    id: row
    spacing: Style.spacing.sm

    IslandButton {
      anchors.verticalCenter: parent.verticalCenter
      glyph: root.leftGlyph
      label: root.leftLabel
      tabbable: false
      size: Style.space(22)
      glyphSize: Style.font.bodySmall
      opacity: root.value > root.from ? 1 : 0.35
      onClicked: root.bump(-1)
    }
    Text {
      anchors.verticalCenter: parent.verticalCenter
      width: Math.max(implicitWidth, Style.space(58))
      horizontalAlignment: Text.AlignHCenter
      text: root.text !== "" ? root.text : root.value + root.suffix
      color: Commons.Color.bar.text
      font.family: Style.font.family
      font.pixelSize: Style.font.bodySmall
      font.bold: true
      font.features: { "tnum": 1 }
    }
    IslandButton {
      anchors.verticalCenter: parent.verticalCenter
      glyph: root.rightGlyph
      label: root.rightLabel
      tabbable: false
      size: Style.space(22)
      glyphSize: Style.font.bodySmall
      opacity: root.value < root.to ? 1 : 0.35
      onClicked: root.bump(1)
    }
  }

  IslandFocusRing { radius: height / 2 }

  WheelHandler {
    acceptedDevices: PointerDevice.Mouse | PointerDevice.TouchPad
    onWheel: (event) => root.bump(event.angleDelta.y > 0 ? 1 : -1)
  }
}
