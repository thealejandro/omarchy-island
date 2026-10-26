import QtQuick
import qs.Commons
import qs.Commons as Commons

// Omarchy's bar indicators as tiles: night light, silence notifications,
// stay awake, screen recording and reminders. Lit tiles are on.
Row {
  id: root

  required property var island

  readonly property int tileWidth: Math.floor((width - (tiles.count - 1) * spacing) / tiles.count)

  spacing: Style.spacing.md

  // States can change behind our back (keybindings, the menu), so re-read
  // them whenever the island opens.
  onVisibleChanged: if (visible) island.refreshToggles()

  Repeater {
    id: tiles
    model: [
      { glyph: "󰔎", label: "nightLight", on: root.island.nightLight, action: "toggleNightLight" },
      { glyph: "󰂛", label: "doNotDisturb", on: root.island.doNotDisturb, action: "toggleDoNotDisturb" },
      { glyph: "󰅶", label: "stayAwake", on: root.island.stayAwake, action: "toggleStayAwake" },
      { glyph: "󰻂", label: root.island.recording ? "stopRecording" : "recording",
        on: root.island.recording, action: "toggleRecording" },
      { glyph: "󰢌", label: "reminders", on: root.island.reminderCount > 0, action: "openReminders" }
    ]

    Rectangle {
      id: tile
      required property var modelData

      width: root.tileWidth
      height: Style.space(48)
      radius: Style.space(12)
      color: modelData.on ? Util.alpha(Commons.Color.accent, mouse.containsMouse ? 0.3 : 0.2)
        : Util.alpha(Commons.Color.bar.text, mouse.containsMouse ? 0.1 : 0.05)
      scale: mouse.pressed ? 0.96 : 1
      Behavior on color { ColorAnimation { duration: 120 } }
      Behavior on scale { NumberAnimation { duration: 120; easing.type: Easing.OutQuad } }

      Column {
        anchors.centerIn: parent
        spacing: Style.spacing.sm

        Glyph {
          anchors.horizontalCenter: parent.horizontalCenter
          text: tile.modelData.glyph
          color: tile.modelData.on ? Commons.Color.accent : Commons.Color.bar.text
          opacity: tile.modelData.on ? 1 : 0.75
          font.family: Style.font.family
          font.pixelSize: Style.font.iconLarge
        }
        Text {
          anchors.horizontalCenter: parent.horizontalCenter
          width: Math.min(implicitWidth, tile.width - 2 * Style.spacing.sm)
          elide: Text.ElideRight
          text: root.island.tr(tile.modelData.label)
          color: tile.modelData.on ? Commons.Color.bar.text : Util.alpha(Commons.Color.bar.text, 0.6)
          font.family: Style.font.family
          font.pixelSize: Style.font.caption
        }
      }

      MouseArea {
        id: mouse
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: root.island[tile.modelData.action]()
      }
    }
  }
}
