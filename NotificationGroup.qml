import QtQuick
import qs.Commons
import qs.Commons as Commons

// The live notifications of one app: a header with its icon, name and count
// (fold it, or close them all), then its cards. Folded, only the newest
// shows.
Column {
  id: root

  required property var island
  required property var group
  property bool folded: false
  property real now: Date.now()
  signal foldToggled()

  readonly property int count: group ? group.rows.length : 0
  readonly property color textColor: Commons.Color.bar.text

  spacing: Style.spacing.sm

  Item {
    width: parent.width
    height: Style.space(22)
    visible: root.count > 1

    Row {
      anchors.left: parent.left
      anchors.verticalCenter: parent.verticalCenter
      spacing: Style.spacing.md
      NotificationIcon {
        anchors.verticalCenter: parent.verticalCenter
        row: root.group.rows[0]
        size: Style.space(16)
      }
      Text {
        anchors.verticalCenter: parent.verticalCenter
        text: root.group.app + "  " + root.count
        color: Util.alpha(root.textColor, 0.72)
        font.family: Style.font.family
        font.pixelSize: Style.font.caption
        font.bold: true
      }
    }
    Row {
      anchors.right: parent.right
      anchors.verticalCenter: parent.verticalCenter
      spacing: Style.spacing.xs
      IslandButton {
        glyph: root.folded ? "󰅀" : "󰅃"
        size: Style.space(22)
        glyphSize: Style.font.bodySmall
        onClicked: root.foldToggled()
      }
      IslandButton {
        glyph: "󰅖"
        size: Style.space(22)
        glyphSize: Style.font.bodySmall
        onClicked: root.island.notify.dismissApp(root.group.app)
      }
    }
  }

  Repeater {
    model: root.folded ? root.group.rows.slice(0, 1) : root.group.rows
    NotificationCard {
      required property var modelData
      island: root.island
      row: modelData
      now: root.now
      showApp: root.count === 1
      width: root.width
    }
  }

  Text {
    visible: root.folded && root.count > 1
    text: root.island.tr("nt_more", root.count - 1)
    color: Util.alpha(root.textColor, 0.6)
    font.family: Style.font.family
    font.pixelSize: Style.font.caption
    leftPadding: Style.spacing.lg
  }
}
