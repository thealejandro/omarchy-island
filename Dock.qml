import QtQuick
import qs.Commons
import qs.Commons as Commons

// One slim row along the bottom of the island: Omarchy's quick toggles (lit
// when on), a few launchers, and the weather on the right. Hovering a button
// shows its name where the weather description sits.
Item {
  id: root

  required property var island

  readonly property bool showToggles: island.togglesMode !== "bar"
  readonly property int buttonSize: Style.space(32)
  readonly property color textColor: Commons.Color.bar.text
  readonly property color dimColor: Util.alpha(Commons.Color.bar.text, 0.6)
  property string hint: ""

  implicitHeight: buttonSize

  // States can change behind our back (keybindings, the menu), so re-read
  // them whenever the island opens.
  onVisibleChanged: if (visible && showToggles) island.refreshToggles()

  component DockButton: Rectangle {
    id: button
    property string glyph: ""
    property string label: ""
    property bool on: false
    signal clicked()

    width: root.buttonSize
    height: root.buttonSize
    radius: Style.space(10)
    color: on ? Util.alpha(Commons.Color.accent, mouse.containsMouse ? 0.3 : 0.2)
      : Util.alpha(root.textColor, mouse.containsMouse ? 0.1 : 0.05)
    scale: mouse.pressed ? 0.92 : 1
    Behavior on color { ColorAnimation { duration: 120 } }
    Behavior on scale { NumberAnimation { duration: 120; easing.type: Easing.OutQuad } }

    Glyph {
      anchors.centerIn: parent
      text: button.glyph
      color: button.on ? Commons.Color.accent : root.textColor
      opacity: button.on || mouse.containsMouse ? 1 : 0.75
      font.family: Style.font.family
      font.pixelSize: Style.font.body + Style.space(3)
    }

    MouseArea {
      id: mouse
      anchors.fill: parent
      hoverEnabled: true
      cursorShape: Qt.PointingHandCursor
      onContainsMouseChanged: {
        if (containsMouse) root.hint = button.label
        else if (root.hint === button.label) root.hint = ""
      }
      onClicked: button.clicked()
    }
  }

  Row {
    id: buttons
    anchors.verticalCenter: parent.verticalCenter
    spacing: Style.spacing.sm

    Repeater {
      model: root.showToggles ? [
        { glyph: "󰔎", label: "nightLight", on: root.island.nightLight, action: "toggleNightLight" },
        { glyph: "󰂛", label: "doNotDisturb", on: root.island.doNotDisturb, action: "toggleDoNotDisturb" },
        { glyph: "󰅶", label: "stayAwake", on: root.island.stayAwake, action: "toggleStayAwake" },
        { glyph: "󰻂", label: root.island.recording ? "stopRecording" : "recording",
          on: root.island.recording, action: "toggleRecording" },
        { glyph: "󰢌", label: "reminders", on: root.island.reminderCount > 0, action: "openReminders" }
      ] : []

      DockButton {
        required property var modelData
        glyph: modelData.glyph
        label: root.island.tr(modelData.label)
        on: modelData.on
        onClicked: root.island[modelData.action]()
      }
    }

    Rectangle {
      visible: root.showToggles
      anchors.verticalCenter: parent.verticalCenter
      width: 1
      height: root.buttonSize - Style.spacing.lg * 2
      color: Util.alpha(root.textColor, 0.15)
    }

    Repeater {
      model: [
        { glyph: "󰹑", label: "screenshot", action: "screenshot" },
        { glyph: "󰈊", label: "colorPicker", action: "pickColor" },
        { glyph: "󰅌", label: "clipboard", action: "openClipboard" },
        { glyph: "󰞅", label: "emoji", action: "openEmoji" }
      ]

      DockButton {
        required property var modelData
        glyph: modelData.glyph
        label: root.island.tr(modelData.label)
        onClicked: root.island[modelData.action]()
      }
    }
  }

  // The weather, or the name of the hovered button.
  Rectangle {
    id: pill
    readonly property bool hasWeather: !!root.island.weather
    // What's left of the row once the buttons are in, inside the padding.
    readonly property real room: root.width - buttons.width - Style.spacing.lg - 2 * Style.spacing.xl
    anchors.right: parent.right
    anchors.verticalCenter: parent.verticalCenter
    height: root.buttonSize
    width: info.implicitWidth + 2 * Style.spacing.xl
    radius: Style.space(10)
    color: Util.alpha(root.textColor, weatherMouse.containsMouse && root.hint === "" ? 0.1 : 0.05)
    visible: hasWeather || root.hint !== ""
    Behavior on color { ColorAnimation { duration: 120 } }

    Row {
      id: info
      anchors.verticalCenter: parent.verticalCenter
      x: Style.spacing.xl
      spacing: Style.spacing.md

      Glyph {
        id: icon
        anchors.verticalCenter: parent.verticalCenter
        visible: root.hint === ""
        text: root.island.weatherGlyph
        color: root.textColor
        font.family: Style.font.family
        font.pixelSize: Style.font.body + Style.space(3)
      }
      Text {
        id: temp
        anchors.verticalCenter: parent.verticalCenter
        visible: root.hint === ""
        text: pill.hasWeather ? root.island.weather.temp : ""
        color: root.textColor
        font.family: Style.font.family
        font.pixelSize: Style.font.body
        font.bold: true
      }
      Text {
        anchors.verticalCenter: parent.verticalCenter
        width: Math.min(implicitWidth, Math.max(0, pill.room
          - (root.hint === "" ? icon.width + temp.width + 2 * info.spacing : 0)))
        elide: Text.ElideRight
        text: root.hint !== "" ? root.hint : (pill.hasWeather ? root.island.weather.desc : "")
        color: root.hint !== "" ? root.textColor : root.dimColor
        font.family: Style.font.family
        font.pixelSize: Style.font.bodySmall
      }
    }

    MouseArea {
      id: weatherMouse
      anchors.fill: parent
      hoverEnabled: true
      enabled: pill.hasWeather
      cursorShape: Qt.PointingHandCursor
      onClicked: root.island.openWeather()
    }
  }
}
