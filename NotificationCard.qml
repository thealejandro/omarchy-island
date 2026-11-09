import QtQuick
import qs.Commons
import qs.Commons as Commons
import "NotificationModel.js" as NM

// One notification on the notifications page: picture, app and time, title,
// the text (three lines until clicked), its buttons, a reply field when the
// sender takes replies, and a close button. Clicking it does what the
// sender asked for (its default action or command), or opens/closes the
// text when there is nothing to do.
Rectangle {
  id: card

  required property var island
  required property var row
  property bool showApp: true
  property real now: Date.now()
  property bool expanded: false

  readonly property var notify: island.notify
  readonly property bool critical: row.urgency === NM.CRITICAL
  // Its own action or command, else (also for earlier ones) the sender's
  // window, site or link.
  readonly property bool clickable: (row.live && (!!row.execArgv || row.hasDefault)) || NM.mayHaveTarget(row)
  readonly property color textColor: Commons.Color.bar.text
  readonly property color dimColor: Util.alpha(Commons.Color.bar.text, 0.72)
  readonly property var codes: row.codes || []
  // The code just copied, shown with a tick for a moment.
  property string copied: ""
  Timer { id: copiedTimer; interval: 2000; onTriggered: card.copied = "" }

  // Picked in the keyboard mode: outlined, and scrolled into view.
  readonly property bool selected: island.keyboardMode && island.keyboardKey === row.key
  onSelectedChanged: if (selected) Qt.callLater(reveal)
  function reveal() {
    var flick = card.parent
    while (flick && flick.contentY === undefined) flick = flick.parent
    if (!flick || !flick.contentItem) return
    var top = card.mapToItem(flick.contentItem, 0, 0).y - Style.spacing.lg
    var bottom = top + card.height + 2 * Style.spacing.lg
    var maxY = Math.max(0, flick.contentHeight - flick.height)
    if (top < flick.contentY) flick.contentY = Math.max(0, top)
    else if (bottom > flick.contentY + flick.height) flick.contentY = Math.min(maxY, bottom - flick.height)
  }

  function when() {
    var r = NM.relativeTime(row.time, now)
    return r.key ? island.tr(r.key, r.arg) : island.locale.toString(r.arg, "d MMM HH:mm")
  }

  implicitHeight: content.implicitHeight + 2 * Style.spacing.lg
  radius: Style.space(10)
  color: Util.alpha(textColor, hover.hovered || selected ? 0.1 : (row.live ? 0.07 : 0.04))
  border.width: selected ? 2 : (critical && row.live ? 1 : 0)
  border.color: selected ? Commons.Color.accent : Util.alpha(Commons.Color.urgent, 0.6)
  Behavior on color { ColorAnimation { duration: 120 } }

  HoverHandler { id: hover }

  MouseArea {
    anchors.fill: parent
    cursorShape: card.clickable || body.truncated || card.expanded ? Qt.PointingHandCursor : Qt.ArrowCursor
    onClicked: {
      if (card.clickable) card.notify.activate(card.row.key)
      else card.expanded = !card.expanded
    }
  }

  Row {
    id: content
    x: Style.spacing.lg
    y: Style.spacing.lg
    width: card.width - 2 * Style.spacing.lg
    spacing: Style.spacing.lg

    NotificationIcon {
      row: card.row
      size: Style.space(card.row.live ? 32 : 26)
      opacity: card.row.live ? 1 : 0.8
    }

    Column {
      id: texts
      width: content.width - x - closeButton.width
      spacing: Style.space(2)

      Text {
        width: parent.width
        elide: Text.ElideRight
        textFormat: Text.PlainText
        text: (card.showApp && NM.senderLabel(card.row) ? NM.senderLabel(card.row) + "  ·  " : "") + card.when()
        color: card.dimColor
        font.family: Style.font.family
        font.pixelSize: Style.font.caption
      }
      Row {
        width: parent.width
        spacing: Style.spacing.sm
        Rectangle {
          visible: !card.row.read
          anchors.verticalCenter: parent.verticalCenter
          width: Style.space(6)
          height: width
          radius: width / 2
          color: card.critical ? Commons.Color.urgent : Commons.Color.accent
        }
        Text {
          width: parent.width - (card.row.read ? 0 : Style.space(6) + parent.spacing)
          elide: Text.ElideRight
          maximumLineCount: 2
          wrapMode: Text.Wrap
          textFormat: Text.PlainText
          text: card.row.summary
          color: card.critical && card.row.live ? Commons.Color.urgent : card.textColor
          font.family: Style.font.family
          font.pixelSize: Style.font.bodySmall
          font.bold: true
        }
      }
      Text {
        id: body
        width: parent.width
        visible: text !== ""
        wrapMode: Text.Wrap
        elide: Text.ElideRight
        maximumLineCount: card.expanded ? 40 : (card.row.live ? 3 : 1)
        textFormat: Text.PlainText
        text: card.row.body
        color: card.dimColor
        font.family: Style.font.family
        font.pixelSize: Style.font.bodySmall
      }

      Flow {
        width: parent.width
        spacing: Style.spacing.sm
        visible: (card.row.live && (card.row.actions.length > 0 || card.codes.length > 0)) || card.row.link !== ""
        topPadding: Style.spacing.sm
        // One-time codes first: copying one is what the card is there for.
        Repeater {
          model: card.row.live ? card.codes : []
          IslandPill {
            required property var modelData
            compact: true
            on: card.copied === modelData
            glyph: card.copied === modelData ? "󰄬" : "󰆏"
            label: card.copied === modelData ? card.island.tr("nt_copied") : card.island.tr("nt_copyCode", modelData)
            onClicked: if (card.notify.copyCode(card.row.key, modelData)) { card.copied = modelData; copiedTimer.restart() }
          }
        }
        Repeater {
          model: card.row.live ? card.row.actions : []
          IslandPill {
            required property var modelData
            compact: true
            label: modelData.text
            onClicked: card.notify.invoke(card.row.key, modelData.id)
          }
        }
        IslandPill {
          visible: card.row.link !== ""
          compact: true
          glyph: "󰌹"
          label: card.island.tr("nt_openLink")
          onClicked: card.notify.openLink(card.row.key)
        }
      }

      Item {
        width: parent.width
        height: reply.height + Style.spacing.sm
        visible: card.row.live && card.row.reply
        IslandField {
          id: reply
          y: Style.spacing.sm
          island: card.island
          width: parent.width
          placeholder: card.row.replyPlaceholder || card.island.tr("nt_reply")
          onAccepted: (t) => card.notify.reply(card.row.key, t)
        }
      }
    }

    Column {
      id: closeButton
      spacing: Style.spacing.xs
      IslandButton {
        glyph: "󰅖"
        size: Style.space(22)
        glyphSize: Style.font.bodySmall
        opacity: hover.hovered ? 1 : 0.55
        onClicked: card.notify.dismiss(card.row.key)
      }
      // The whole text, when the card's click goes elsewhere.
      IslandButton {
        visible: card.clickable && (body.truncated || card.expanded)
        glyph: card.expanded ? "󰅃" : "󰅀"
        size: Style.space(22)
        glyphSize: Style.font.bodySmall
        opacity: hover.hovered ? 1 : 0.55
        onClicked: card.expanded = !card.expanded
      }
    }
  }
}
