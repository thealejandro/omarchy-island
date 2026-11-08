import QtQuick
import Quickshell
import qs.Commons
import qs.Commons as Commons

// A notification's picture: the image it carried, else its app's icon from
// the icon theme or desktop entry, else a bell. Decoded small and never
// cached, so a big picture never sits in memory at full size.
Rectangle {
  id: root

  property var row: null
  property int size: Style.space(32)

  readonly property string source: {
    var r = row
    if (!r) return ""
    if (r.image) return r.image
    if (r.iconFile) return r.iconFile
    var names = [r.appIcon, r.desktopEntry, String(r.app || "").toLowerCase().replace(/[^a-z0-9._-]+/g, "-")]
    for (var i = 0; i < names.length; i++) {
      if (!names[i]) continue
      var path = Quickshell.iconPath(names[i], true)
      if (path) return path
    }
    try {
      var entry = DesktopEntries.heuristicLookup(r.desktopEntry || r.app || "")
      if (entry && entry.icon) return Quickshell.iconPath(entry.icon, true)
    } catch (e) { }
    return ""
  }
  readonly property bool critical: !!row && row.urgency === 2

  width: size
  height: size
  radius: Style.space(8)
  color: image.status === Image.Ready ? "transparent"
    : Util.alpha(critical ? Commons.Color.urgent : Commons.Color.accent, 0.14)
  clip: true

  Image {
    id: image
    anchors.fill: parent
    source: root.source
    sourceSize: Qt.size(root.size * 2, root.size * 2)
    fillMode: Image.PreserveAspectCrop
    asynchronous: true
    cache: false
    smooth: true
    visible: status === Image.Ready
  }
  Glyph {
    anchors.centerIn: parent
    visible: image.status !== Image.Ready
    text: root.critical ? "󰀦" : "󰂚"
    color: root.critical ? Commons.Color.urgent : Commons.Color.accent
    font.family: Style.font.family
    font.pixelSize: Math.round(root.size * 0.55)
  }
}
