import QtQuick
import qs.Commons
import qs.Commons as Commons
import "IslandModel.js" as IslandModel

// The open island's pages as a slim row of glyphs; the current one is lit.
// One Tab stop, on the current page's tab; ← and → change the page.
Row {
  id: root

  required property var island

  readonly property bool quiet: !!island.notify && island.notify.doNotDisturb
  readonly property var glyphs: ({ home: "󰋜", focus: "󰔛", calendar: "󰃭", world: "󰇧",
    notifications: quiet ? "󰂛" : "󰂚", settings: "󰒓" })

  spacing: Style.spacing.xs
  Accessible.role: Accessible.PageTabList
  Accessible.name: island.tr("a11y_pages")

  // The neighbouring page, keeping the keyboard on its tab.
  function go(delta) {
    var next = IslandModel.stepChoice(island.pages, island.page, delta)
    if (next === island.page) return
    island.page = next
    var tab = tabs.itemAt(island.pages.indexOf(next))
    if (tab) tab.forceActiveFocus()
  }

  Repeater {
    id: tabs
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
      activeFocusOnTab: active
      Keys.onLeftPressed: root.go(-1)
      Keys.onRightPressed: root.go(1)
      Accessible.role: Accessible.PageTab
      Accessible.name: root.island.tr("page_" + modelData)
      Accessible.checkable: true
      Accessible.checked: active
      Accessible.onPressAction: root.island.page = tab.modelData

      Glyph {
        anchors.centerIn: parent
        text: root.glyphs[tab.modelData] || "?"
        color: tab.active ? Commons.Color.accent : Commons.Color.bar.text
        opacity: tab.active || mouse.containsMouse ? 1 : 0.72
        font.family: Style.font.family
        font.pixelSize: Style.font.body + Style.space(1)
      }
      // Unread notifications.
      Rectangle {
        visible: tab.modelData === "notifications" && root.island.cfg.notifyBadge
          && root.island.unread > 0 && !tab.active
        width: Style.space(6)
        height: width
        radius: width / 2
        x: parent.width - width - Style.space(3)
        y: Style.space(2)
        color: Commons.Color.accent
      }
      IslandFocusRing { radius: tab.radius + Style.space(2) }
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
