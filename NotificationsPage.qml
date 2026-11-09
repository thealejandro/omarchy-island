import QtQuick
import qs.Commons
import qs.Commons as Commons
import "NotificationModel.js" as NM

// Notifications: what is still open, grouped by app (by site for a browser),
// then the earlier ones.
// Silence, mark read, close all and clear the history along the top.
Column {
  id: root

  required property var island
  readonly property var notify: island.notify
  readonly property var live: notify ? notify.liveRows : []
  readonly property var earlier: notify ? notify.historyRows : []
  readonly property var groups: NM.groupByApp(live)
  readonly property color textColor: Commons.Color.bar.text
  readonly property color dimColor: Util.alpha(Commons.Color.bar.text, 0.72)
  // Groups folded by hand; big groups start folded.
  property var folds: ({})
  property real now: Date.now()
  // The earlier ones shown; the rest stay in the history file until they
  // age out.
  readonly property int earlierShown: 30
  // The cards top to bottom, for the keyboard mode.
  readonly property var order: NM.keyboardOrder(groups, folds, earlier, earlierShown)
  onOrderChanged: island.keyboardOrder = order
  Component.onCompleted: island.keyboardOrder = order

  spacing: Style.spacing.lg

  Timer {
    interval: 30000
    repeat: true
    running: root.visible && root.island.isOpen
    onTriggered: root.now = Date.now()
  }

  // Off, or standing by: say so instead of an empty list.
  Text {
    width: parent.width
    visible: !root.notify
    wrapMode: Text.WordWrap
    text: root.island.tr(root.island.notificationsStandby ? "note_notifyStandby" : "note_notifyOff")
    color: root.dimColor
    font.family: Style.font.family
    font.pixelSize: Style.font.bodySmall
  }

  Flow {
    width: parent.width
    visible: !!root.notify
    spacing: Style.spacing.sm
    IslandPill {
      glyph: "󰂛"
      label: root.island.tr("nt_dnd")
      on: !!root.notify && root.notify.doNotDisturb
      onClicked: root.island.toggleDoNotDisturb()
    }
    IslandPill {
      visible: !!root.notify && root.notify.unread > 0
      label: root.island.tr("nt_markRead")
      onClicked: root.notify.markAllRead()
    }
    IslandPill {
      visible: root.live.length > 0
      label: root.island.tr("nt_dismissAll")
      onClicked: root.notify.dismissAll()
    }
    IslandPill {
      visible: root.earlier.length > 0
      label: root.island.tr("nt_clear")
      onClicked: root.notify.clearHistory()
    }
  }

  // Nothing at all.
  Column {
    width: parent.width
    visible: !!root.notify && root.live.length === 0 && root.earlier.length === 0
    spacing: Style.spacing.sm
    topPadding: Style.spacing.lg
    bottomPadding: Style.spacing.lg
    Glyph {
      anchors.horizontalCenter: parent.horizontalCenter
      text: "󰂚"
      color: Util.alpha(Commons.Color.accent, 0.8)
      font.family: Style.font.family
      font.pixelSize: Style.font.icon + Style.space(10)
    }
    Text {
      anchors.horizontalCenter: parent.horizontalCenter
      text: root.island.tr("nt_empty")
      color: root.textColor
      font.family: Style.font.family
      font.pixelSize: Style.font.body
      font.bold: true
    }
    Text {
      width: parent.width
      horizontalAlignment: Text.AlignHCenter
      wrapMode: Text.WordWrap
      text: root.island.tr("nt_emptyHint")
      color: root.dimColor
      font.family: Style.font.family
      font.pixelSize: Style.font.caption
    }
  }

  Repeater {
    model: root.groups
    NotificationGroup {
      required property var modelData
      island: root.island
      group: modelData
      width: root.width
      now: root.now
      folded: NM.isFolded(root.folds, modelData)
      onFoldToggled: {
        var next = ({})
        for (var k in root.folds) next[k] = root.folds[k]
        next[modelData.key] = !folded
        root.folds = next
      }
    }
  }

  Text {
    visible: root.earlier.length > 0
    topPadding: root.live.length > 0 ? Style.spacing.sm : 0
    text: root.island.tr("nt_earlier")
    color: root.dimColor
    font.family: Style.font.family
    font.pixelSize: Style.font.caption
    font.bold: true
    font.capitalization: Font.AllUppercase
    font.letterSpacing: 0.6
  }

  Repeater {
    model: root.earlier.slice(0, root.earlierShown)
    NotificationCard {
      required property var modelData
      island: root.island
      row: modelData
      now: root.now
      width: root.width
    }
  }
}
