import QtQuick
import qs.Commons
import qs.Commons as Commons

// The open island: a slim header with the page's title and the page tabs,
// then the page itself. Home is built in; the other pages load on demand,
// so a page that fails to load leaves a line of text instead of breaking
// the island. Pages taller than the room the island has scroll.
Column {
  id: root

  required property var island
  property bool shown: false
  // Room the island has for this view; taller pages scroll.
  property real maxHeight: 10000

  readonly property color textColor: Commons.Color.bar.text
  readonly property color dimColor: Util.alpha(Commons.Color.bar.text, 0.72)
  readonly property var sources: ({
    calendar: "CalendarPage.qml",
    world: "WorldPage.qml",
    settings: "SettingsView.qml"
  })
  readonly property string page: island.page
  readonly property bool onHome: page === "home" || !sources[page]

  spacing: Style.spacing.lg

  Item {
    id: header
    width: parent.width
    height: tabs.height
    opacity: root.shown ? 1 : 0
    Behavior on opacity { NumberAnimation { duration: 260 } }

    Text {
      anchors.left: parent.left
      anchors.right: tabs.left
      anchors.rightMargin: Style.spacing.lg
      anchors.verticalCenter: parent.verticalCenter
      elide: Text.ElideRight
      text: root.island.tabHint !== "" ? root.island.tabHint
        : (root.onHome ? root.island.locale.toString(new Date(), "dddd d MMMM")
          : root.island.tr("page_" + root.page))
      color: root.dimColor
      font.family: Style.font.family
      font.pixelSize: Style.font.caption
      font.bold: true
      font.capitalization: Font.AllUppercase
      font.letterSpacing: 0.6
    }

    PageTabs {
      id: tabs
      island: root.island
      anchors.right: parent.right
    }
  }

  Flickable {
    id: flick
    width: parent.width
    height: Math.min(content.height, Math.max(0, root.maxHeight - header.height - root.spacing))
    contentWidth: width
    contentHeight: content.height
    clip: contentHeight > height
    interactive: contentHeight > height
    boundsBehavior: Flickable.StopAtBounds

    Item {
      id: content
      width: flick.width
      height: root.onHome ? home.height : (loader.item ? loader.item.height : fallback.height)

      HomePage {
        id: home
        island: root.island
        width: parent.width
        visible: root.onHome
        shown: root.shown && root.onHome
      }

      Loader {
        id: loader
        width: parent.width
        active: !root.onHome
        visible: !root.onHome && status === Loader.Ready
        opacity: 0
        function load() {
          if (!active || !root.sources[root.page]) return
          var url = Qt.resolvedUrl(root.sources[root.page])
          if (String(source) === String(url)) return
          setSource(String(url), { "island": root.island })
        }
        onActiveChanged: load()
        onLoaded: { item.width = Qt.binding(function() { return loader.width }); opacity = 0; fadeIn.restart() }
        NumberAnimation on opacity { id: fadeIn; running: false; to: 1; duration: 220; easing.type: Easing.OutCubic }
      }

      Text {
        id: fallback
        width: parent.width
        visible: !root.onHome && loader.status === Loader.Error
        wrapMode: Text.WordWrap
        text: root.island.tr("page_missing")
        color: root.dimColor
        font.family: Style.font.family
        font.pixelSize: Style.font.bodySmall
      }
    }
  }

  onPageChanged: { flick.contentY = 0; loader.load() }
}
