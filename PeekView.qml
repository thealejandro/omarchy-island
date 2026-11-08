import QtQuick
import qs.Commons
import qs.Commons as Commons
import "IslandModel.js" as IslandModel

// The glance: one line saying what just changed. Media and the timer show
// their track or countdown; Omarchy's OSD and the battery show an icon with
// a message or a level bar.
Row {
  id: root

  required property var island

  // "notification" | "osd" | "battery" | "agenda" | "timer" | "media"
  readonly property var note: island.peekReason === "notification" && island.notify ? island.notify.current : null
  readonly property string kind: note ? "notification"
    : island.peekReason === "osd" && island.osd ? "osd"
    : island.peekReason === "agenda" && island.agendaNote ? "agenda"
    : island.peekReason === "battery" && island.hasBattery ? "battery"
    : island.peekReason === "timer" || (!island.hasMedia && island.timerActive) ? "timer"
    : "media"
  readonly property bool showTimer: kind === "timer"
  readonly property bool indicator: kind === "osd" || kind === "battery" || kind === "agenda"
  readonly property bool showBar: kind === "osd" ? island.osd.hasProgress : kind === "battery"
  readonly property real level: kind === "osd" ? island.osd.progress
    : kind === "battery" ? island.batteryLevel / 100 : 0
  readonly property bool batteryLow: (kind === "battery" && island.batteryNote === "batteryLow")
    || (kind === "notification" && note.urgency === 2)
  // Others from the same app that came in right behind this one, and the
  // rest still waiting their turn.
  readonly property int more: kind === "notification" ? island.notify.burst + island.notify.queue.length : 0
  readonly property int thumbSize: Style.space(22)

  readonly property string noteText: !note ? ""
    : (note.summary || note.app) + (island.cfg.notifyBody && note.body
      ? "  ·  " + note.body.replace(/\s*\n\s*/g, "  ") : "")

  spacing: Style.spacing.lg
  height: thumbSize

  // A new notification slides in from just above, short and light.
  property real enter: 1
  onNoteChanged: if (note && island.animate) { enter = 0; enterAnim.restart() }
  NumberAnimation { id: enterAnim; target: root; property: "enter"; to: 1; duration: 220; easing.type: Easing.OutCubic }
  opacity: kind === "notification" ? enter : 1
  transform: Translate { y: root.kind === "notification" ? (1 - root.enter) * -Style.space(6) : 0 }

  Rectangle {
    anchors.verticalCenter: parent.verticalCenter
    width: root.thumbSize
    height: root.thumbSize
    radius: Style.space(6)
    color: root.indicator ? "transparent" : Util.alpha(Commons.Color.bar.text, 0.08)
    clip: true
    visible: root.kind !== "notification"

    Image {
      anchors.fill: parent
      visible: root.kind === "media" && status === Image.Ready
      source: root.kind === "media" ? root.island.artUrl : ""
      sourceSize: Qt.size(64, 64)
      fillMode: Image.PreserveAspectCrop
      asynchronous: true
    }
    Glyph {
      anchors.centerIn: parent
      visible: root.kind !== "media" || root.island.artUrl === ""
      text: root.kind === "notification" ? root.noteText
      : root.kind === "osd" ? root.island.osd.icon
        : root.kind === "battery" ? IslandModel.batteryIcon(root.island.batteryLevel, root.island.charging)
        : root.kind === "agenda" ? "󰃰"
        : root.showTimer ? "󰔛" : "󰝚"
      color: root.batteryLow ? Commons.Color.urgent : Commons.Color.accent
      font.family: Style.font.family
      font.pixelSize: Style.font.icon
    }
  }

  NotificationIcon {
    anchors.verticalCenter: parent.verticalCenter
    visible: root.kind === "notification"
    row: root.note
    size: root.thumbSize
  }

  // Volume, brightness and battery level. The fill glides between presses
  // so holding a key reads as one motion.
  Item {
    anchors.verticalCenter: parent.verticalCenter
    visible: root.showBar
    width: Style.space(150)
    height: Style.space(5)
    Rectangle {
      anchors.fill: parent
      radius: height / 2
      color: Util.alpha(Commons.Color.bar.text, 0.15)
    }
    Rectangle {
      height: parent.height
      width: parent.width * Math.max(0, Math.min(1, root.level))
      radius: height / 2
      color: root.batteryLow ? Commons.Color.urgent : Commons.Color.accent
      Behavior on width { NumberAnimation { duration: 160; easing.type: Easing.OutCubic } }
    }
  }

  Text {
    anchors.verticalCenter: parent.verticalCenter
    width: Math.min(implicitWidth, Style.space(root.kind === "notification" ? 360 : 320))
    elide: Text.ElideRight
    textFormat: Text.PlainText
    text: root.kind === "notification" ? root.noteText
      : root.kind === "osd" ? (root.island.osd.hasProgress ? root.island.osd.label : root.island.osd.message)
      : root.kind === "battery" ? root.island.tr(root.island.batteryNote) + "  " + root.island.batteryLevel + "%"
      : root.kind === "agenda" ? root.island.agendaNote.title + "  ·  " + (root.island.agendaNote.minutes > 0
        ? root.island.tr("eventIn", root.island.agendaNote.minutes) : root.island.tr("eventNow"))
      : root.showTimer ? root.island.tr(root.island.timerPhase) + "  " + IslandModel.clock(root.island.timerRemaining)
      : root.island.title + (root.island.artist ? "  ·  " + root.island.artist : "")
    color: Commons.Color.bar.text
    font.family: Style.font.family
    font.pixelSize: Style.font.body
    font.bold: true
    font.features: { "tnum": 1 }
  }

  Text {
    anchors.verticalCenter: parent.verticalCenter
    visible: root.more > 0
    text: root.island.tr("nt_more", root.more)
    color: Util.alpha(Commons.Color.bar.text, 0.6)
    font.family: Style.font.family
    font.pixelSize: Style.font.caption
    font.bold: true
  }

  Equalizer {
    anchors.verticalCenter: parent.verticalCenter
    visible: root.kind === "media"
    playing: root.island.playing
  }
}
