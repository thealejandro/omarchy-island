import QtQuick
import qs.Commons
import qs.Commons as Commons
import "IslandModel.js" as IslandModel

Column {
  id: root
  required property var island
  spacing: Style.spacing.xl

  Text {
    width: parent.width
    text: root.island.tr(root.island.timerPhase)
    textFormat: Text.PlainText
    horizontalAlignment: Text.AlignHCenter
    color: Commons.Color.bar.text
    font.family: Style.font.family
    font.pixelSize: Style.font.heading
    font.bold: true
  }
  Text {
    width: parent.width
    text: root.island.tr("focusRound", Math.min(root.island.timerRounds + (root.island.timerPhase === "focus" ? 1 : 0), root.island.cfg.longBreakEvery))
      + " / " + root.island.cfg.longBreakEvery
    textFormat: Text.PlainText
    horizontalAlignment: Text.AlignHCenter
    color: Util.alpha(Commons.Color.bar.text, 0.6)
    font.family: Style.font.family
    font.pixelSize: Style.font.caption
  }
  TimerRow {
    width: parent.width
    island: root.island
  }
  Row {
    anchors.horizontalCenter: parent.horizontalCenter
    spacing: Style.spacing.md
    Repeater {
      model: root.island.cfg.longBreakEvery
      Rectangle {
        required property int index
        width: Style.space(8)
        height: width
        radius: width / 2
        color: index < root.island.timerRounds ? Commons.Color.accent : Util.alpha(Commons.Color.bar.text, 0.2)
      }
    }
  }
}
