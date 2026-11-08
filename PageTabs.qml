import QtQuick
import qs.Commons
import qs.Commons as Commons

// The open island's pages as a slim row of glyphs; the current one is lit.
Row {
  id: root

  required property var island

  readonly property var glyphs: ({ home: "󰋜", calendar: "󰃭", world: "󰇧", settings: "󰒓" })

  spacing: Style.spacing.xs

  Repeater {
    model: root.island.pages

    Rectangle {
      id: tab
      required property string modelData
      readonly property bool active: root.island.page === modelData
      width: Style.space(26)
      height: Style.space(22)
      radius: Style.space(7)
      color: active ? Util.alpha(Commons.Color.accent, 0.2)
        : Util.alpha(Commons.Color.bar.text, mouse.containsMouse ? 0.1 : 0)
      Behavior on color { ColorAnimation { duration: 140 } }

      Glyph {
        anchors.centerIn: parent
        text: root.glyphs[tab.modelData] || "?"
        color: tab.active ? Commons.Color.accent : Commons.Color.bar.text
        opacity: tab.active || mouse.containsMouse ? 1 : 0.72
        font.family: Style.font.family
        font.pixelSize: Style.font.body + Style.space(1)
      }
      MouseArea {
        id: mouse
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: root.island.page = tab.modelData
        onContainsMouseChanged: root.island.tabHint = containsMouse ? root.island.tr("page_" + tab.modelData) : ""
      }
    }
  }
}
