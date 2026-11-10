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
    focus: "FocusPage.qml",
    calendar: "CalendarPage.qml",
    world: "WorldPage.qml",
    notifications: "NotificationsPage.qml",
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
    Behavior on opacity { enabled: root.island.animate; NumberAnimation { duration: 260 } }

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
      // A page change slides the new page in a little from the side of its
      // tab, with a short fade; transforms and opacity only.
      property real slide: 0
      opacity: 1
      transform: Translate { x: content.slide }
      ParallelAnimation {
        id: pageIn
        NumberAnimation { target: content; property: "slide"; to: 0; duration: 200; easing.type: Easing.OutCubic }
        NumberAnimation { target: content; property: "opacity"; to: 1; duration: 160; easing.type: Easing.OutCubic }
      }
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
        // Opening straight onto a page (the clock's calendar, the weather's
        // world) builds it over a few frames while the island grows, not
        // in the frame the click lands; a tab change while open is at once.
        asynchronous: !root.shown
        visible: !root.onHome && status === Loader.Ready
        function load() {
          if (!active || !root.sources[root.page]) return
          var url = Qt.resolvedUrl(root.sources[root.page])
          if (String(source) === String(url)) return
          setSource(String(url), { "island": root.island })
        }
        onActiveChanged: load()
        onLoaded: item.width = Qt.binding(function() { return loader.width })
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

  property string lastPage: page
  onPageChanged: {
    flick.contentY = 0
    loader.load()
    var pages = island.pages
    var dir = pages.indexOf(page) >= pages.indexOf(lastPage) ? 1 : -1
    lastPage = page
    if (!island.animate || !shown) return
    content.slide = dir * Style.space(18)
    content.opacity = 0
    pageIn.restart()
  }
}
