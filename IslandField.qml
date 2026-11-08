import QtQuick
import qs.Commons
import qs.Commons as Commons

// A one-line text field. The island only takes the keyboard while one of
// these is being edited (island.wantsKeyboard), and gives it back on Enter,
// Escape or a click elsewhere. Enter commits; Escape puts the text back.
Rectangle {
  id: root

  required property var island
  property string text: ""
  property string placeholder: ""
  property bool editing: input.activeFocus
  signal accepted(string text)

  implicitWidth: Style.space(220)
  implicitHeight: Style.space(26)
  radius: Style.space(8)
  color: Util.alpha(Commons.Color.bar.text, editing ? 0.1 : (mouse.containsMouse ? 0.09 : 0.06))
  border.width: editing ? 1 : 0
  border.color: Util.alpha(Commons.Color.accent, 0.7)

  onTextChanged: if (!editing) input.text = text
  Component.onCompleted: input.text = text

  function begin() {
    island.wantsKeyboard = true
    // The layer gets keyboard focus a moment after asking for it.
    Qt.callLater(function() { input.forceActiveFocus(); input.selectAll() })
  }
  function finish(commit) {
    if (commit) root.accepted(input.text)
    else input.text = root.text
    input.focus = false
    island.wantsKeyboard = false
  }

  // Someone else (closing the island) took the keyboard back.
  Connections {
    target: root.island
    function onWantsKeyboardChanged() { if (!root.island.wantsKeyboard && input.activeFocus) input.focus = false }
  }

  TextInput {
    id: input
    anchors.fill: parent
    anchors.leftMargin: Style.spacing.lg
    anchors.rightMargin: Style.spacing.lg
    verticalAlignment: TextInput.AlignVCenter
    clip: true
    color: Commons.Color.bar.text
    selectionColor: Util.alpha(Commons.Color.accent, 0.4)
    selectedTextColor: Commons.Color.bar.text
    font.family: Style.font.family
    font.pixelSize: Style.font.bodySmall
    selectByMouse: true
    Keys.onReturnPressed: root.finish(true)
    Keys.onEnterPressed: root.finish(true)
    Keys.onEscapePressed: root.finish(false)
    onActiveFocusChanged: if (!activeFocus && root.island.wantsKeyboard && root.editing === false) root.island.wantsKeyboard = false

    Text {
      anchors.verticalCenter: parent.verticalCenter
      visible: input.text === "" && !input.activeFocus
      text: root.placeholder
      color: Util.alpha(Commons.Color.bar.text, 0.5)
      font: input.font
      elide: Text.ElideRight
      width: parent.width
    }
  }

  MouseArea {
    id: mouse
    anchors.fill: parent
    hoverEnabled: true
    cursorShape: Qt.IBeamCursor
    visible: !input.activeFocus
    onClicked: root.begin()
  }
}
