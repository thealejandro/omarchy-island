import QtQuick
import qs.Commons
import qs.Commons as Commons
import "AgentModel.js" as AgentModel
import "MediaModel.js" as MediaModel

// The island's first page, for a glance: now playing next to what's next
// on the agenda, the newest notifications when the island shows them, the
// focus timer, and a dock of toggles, launchers and the weather along the
// bottom. The month and the world clocks have their own pages. Each part
// can be switched off in the settings.
Column {
  id: root

  required property var island
  property bool shown: false

  readonly property var cfg: island.cfg
  readonly property int calendarWidth: Style.space(196)
  readonly property color lineColor: Util.alpha(Commons.Color.bar.text, 0.2)
  readonly property int rise: Style.space(10)
  readonly property real lyricsWidth: MediaModel.lyricsWidth(width, Style.spacing.xxl,
    Style.space(180), Style.space(160))
  readonly property bool lyricsBeside: cfg.showMedia && cfg.lyrics && cfg.openLyrics && lyricsWidth > 0

  // One clock for the whole entrance. Each section starts a little after the
  // one above it, so the content cascades down as the body settles.
  property real progress: shown ? 1 : 0
  Behavior on progress { enabled: root.island.animate; NumberAnimation { duration: Style.duration(root.shown ? 240 : 120) } }

  function reveal(index) {
    var t = Math.max(0, Math.min(1, (progress - index * 0.12) / 0.64))
    return 1 - Math.pow(1 - t, 3)
  }

  spacing: Style.spacing.xl

  Row {
    id: top
    width: parent.width
    spacing: Style.spacing.xxl
    layoutDirection: !root.lyricsBeside && root.cfg.calendarFirst ? Qt.RightToLeft : Qt.LeftToRight
    opacity: root.reveal(0)
    transform: [
      Scale {
        origin.x: top.width / 2
        xScale: 0.96 + 0.04 * root.reveal(0)
        yScale: xScale
      },
      Translate { y: (1 - root.reveal(0)) * root.rise }
    ]

    MediaCard {
      id: media
      visible: root.cfg.showMedia
      island: root.island
      width: root.lyricsBeside ? parent.width - root.lyricsWidth - parent.spacing
        : Math.max(0, parent.width - root.calendarWidth - 2 * parent.spacing - 1)
      inlineLyrics: !root.lyricsBeside
      anchors.verticalCenter: parent.verticalCenter
    }

    Rectangle {
      visible: root.cfg.showMedia && !root.lyricsBeside
      width: 1
      height: parent.height
      color: root.lineColor
    }

    TodayCard {
      visible: !root.lyricsBeside
      island: root.island
      width: root.cfg.showMedia ? root.calendarWidth : top.width
      anchors.verticalCenter: parent.verticalCenter
    }

    Item {
      visible: root.lyricsBeside
      width: root.lyricsWidth
      height: Math.max(Style.space(152), media.height)
      anchors.verticalCenter: parent.verticalCenter
      LyricsColumn {
        anchors.fill: parent
        island: root.island
        visible: root.island.lyricsState === "synced"
      }
      Text {
        anchors.centerIn: parent
        width: parent.width
        visible: root.island.hasMedia && root.island.lyricsState !== "synced"
        text: root.island.tr(root.island.lyricsState === "loading" ? "lyricsLoading" : "lyricsUnavailable")
        textFormat: Text.PlainText
        wrapMode: Text.WordWrap
        horizontalAlignment: Text.AlignHCenter
        color: Util.alpha(Commons.Color.bar.text, 0.5)
        font.family: Style.font.family
        font.pixelSize: Style.font.caption
      }
    }
  }

  TodayCard {
    visible: root.lyricsBeside
    island: root.island
    width: parent.width
    opacity: root.reveal(1)
  }

  // The newest notifications still open, when the island shows them. A
  // click goes to the notifications page.
  Column {
    id: recent
    readonly property var rows: root.island.notify ? root.island.notify.liveRows : []
    visible: root.island.notificationsLive && rows.length > 0
    width: parent.width
    spacing: Style.spacing.xs
    opacity: root.reveal(1)
    transform: Translate { y: (1 - root.reveal(1)) * root.rise }

    Repeater {
      model: recent.rows.slice(0, 2)
      Rectangle {
        id: note
        required property var modelData
        required property int index
        width: recent.width
        height: Style.space(30)
        radius: Style.space(8)
        color: Util.alpha(Commons.Color.bar.text, noteMouse.containsMouse ? 0.1 : 0.05)
        Behavior on color { ColorAnimation { duration: 120 } }
        Row {
          anchors.left: parent.left
          anchors.right: parent.right
          anchors.leftMargin: Style.spacing.md
          anchors.rightMargin: Style.spacing.md
          anchors.verticalCenter: parent.verticalCenter
          spacing: Style.spacing.md
          NotificationIcon {
            anchors.verticalCenter: parent.verticalCenter
            row: note.modelData
            size: Style.space(18)
          }
          Text {
            anchors.verticalCenter: parent.verticalCenter
            width: parent.width - Style.space(18) - more.width - 2 * parent.spacing
            elide: Text.ElideRight
            textFormat: Text.PlainText
            text: (note.modelData.app ? note.modelData.app + "  ·  " : "") + (note.modelData.summary || note.modelData.body)
            color: note.modelData.urgency === 2 ? Commons.Color.urgent : Commons.Color.bar.text
            font.family: Style.font.family
            font.pixelSize: Style.font.bodySmall
            font.bold: !note.modelData.read
          }
          Text {
            id: more
            anchors.verticalCenter: parent.verticalCenter
            visible: note.index === 0 && recent.rows.length > 2
            width: visible ? implicitWidth : 0
            text: root.island.tr("nt_more", recent.rows.length - 2)
            color: Util.alpha(Commons.Color.bar.text, 0.6)
            font.family: Style.font.family
            font.pixelSize: Style.font.caption
            font.bold: true
          }
        }
        MouseArea {
          id: noteMouse
          anchors.fill: parent
          hoverEnabled: true
          cursorShape: Qt.PointingHandCursor
          onClicked: root.island.showPage("notifications")
        }
      }
    }
  }

  // AI agents that need you, are working or just finished (setting
  // `agents`); the newest two. A click raises the agent's window when its
  // pid or address is known.
  Column {
    id: agents
    readonly property var rows: root.cfg.agents ? AgentModel.ordered(root.island.agentSessions) : []
    visible: rows.length > 0
    width: parent.width
    spacing: Style.spacing.xs
    opacity: root.reveal(1)
    transform: Translate { y: (1 - root.reveal(1)) * root.rise }

    Repeater {
      model: agents.rows.slice(0, 2)
      Rectangle {
        id: agent
        required property var modelData
        required property int index
        readonly property bool known: modelData.pid > 0 || modelData.address !== ""
        readonly property bool waiting: modelData.state === "waiting"
        readonly property string title: AgentModel.sessionLabel(modelData)
        readonly property string status: root.island.tr("agent_" + modelData.state)
        width: agents.width
        height: Style.space(30)
        radius: Style.space(8)
        color: Util.alpha(Commons.Color.bar.text, agentMouse.containsMouse ? 0.1 : 0.05)
        Behavior on color { ColorAnimation { duration: 120 } }
        Accessible.role: Accessible.Button
        Accessible.name: root.island.tr("a11y_agentRow", agent.title + ", " + agent.status)
        Accessible.onPressAction: if (agent.known) root.island.agentFocus(agent.modelData)
        Row {
          anchors.left: parent.left
          anchors.right: parent.right
          anchors.leftMargin: Style.spacing.md
          anchors.rightMargin: Style.spacing.md
          anchors.verticalCenter: parent.verticalCenter
          spacing: Style.spacing.md
          Item {
            anchors.verticalCenter: parent.verticalCenter
            width: Style.space(18)
            height: Style.space(18)
            Glyph {
              anchors.centerIn: parent
              text: AgentModel.glyph(agent.modelData.state)
              color: agent.waiting || agent.modelData.state === "error" ? Commons.Color.urgent : Commons.Color.accent
              font.family: Style.font.family
              font.pixelSize: Style.font.icon
            }
          }
          Text {
            anchors.verticalCenter: parent.verticalCenter
            width: parent.width - Style.space(18) - statusText.width - extra.width - 3 * parent.spacing
            elide: Text.ElideRight
            textFormat: Text.PlainText
            text: agent.title
            color: agent.waiting ? Commons.Color.urgent : Commons.Color.bar.text
            font.family: Style.font.family
            font.pixelSize: Style.font.bodySmall
            font.bold: agent.waiting
          }
          Text {
            id: statusText
            anchors.verticalCenter: parent.verticalCenter
            textFormat: Text.PlainText
            text: agent.status
            color: Util.alpha(Commons.Color.bar.text, 0.6)
            font.family: Style.font.family
            font.pixelSize: Style.font.caption
            font.bold: agent.waiting
          }
          Text {
            id: extra
            anchors.verticalCenter: parent.verticalCenter
            visible: agent.index === 0 && agents.rows.length > 2
            width: visible ? implicitWidth : 0
            text: root.island.tr("nt_more", agents.rows.length - 2)
            color: Util.alpha(Commons.Color.bar.text, 0.6)
            font.family: Style.font.family
            font.pixelSize: Style.font.caption
            font.bold: true
          }
        }
        MouseArea {
          id: agentMouse
          anchors.fill: parent
          hoverEnabled: true
          cursorShape: agent.known ? Qt.PointingHandCursor : Qt.ArrowCursor
          onClicked: if (agent.known) root.island.agentFocus(agent.modelData)
        }
      }
    }
  }

  Rectangle {
    visible: root.cfg.showTimer || root.cfg.showDock
    width: parent.width
    height: 1
    color: root.lineColor
    opacity: root.reveal(1)
    transform: Scale { origin.x: root.width / 2; xScale: root.reveal(1) }
  }

  TimerRow {
    visible: root.cfg.showTimer
    island: root.island
    width: parent.width
    opacity: root.reveal(2)
    transform: Translate { y: (1 - root.reveal(2)) * root.rise }
  }

  Dock {
    visible: root.cfg.showDock
    island: root.island
    width: parent.width
    opacity: root.reveal(3)
    transform: Translate { y: (1 - root.reveal(3)) * root.rise }
  }
}
