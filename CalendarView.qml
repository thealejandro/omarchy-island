import QtQuick
import qs.Commons
import qs.Commons as Commons
import "IslandModel.js" as IslandModel

// Month at a glance. Arrows or the scroll wheel page through months; the
// header (or "today") jumps back to the current one.
Column {
  id: root

  required property var island

  readonly property var locale: island.locale
  readonly property color textColor: Commons.Color.bar.text
  readonly property color dimColor: Util.alpha(Commons.Color.bar.text, 0.5)
  readonly property int cell: Math.floor(width / 7)

  property date today: new Date()
  property int year: today.getFullYear()
  property int month: today.getMonth()
  readonly property bool onCurrentMonth: year === today.getFullYear() && month === today.getMonth()

  function shift(delta) {
    var d = new Date(year, month + delta, 1)
    year = d.getFullYear()
    month = d.getMonth()
    slide(delta)
  }
  function goToday() {
    today = new Date()
    var delta = (today.getFullYear() - year) * 12 + today.getMonth() - month
    year = today.getFullYear()
    month = today.getMonth()
    if (delta !== 0 && visible) slide(delta)
  }
  // The new month slides in from the side it came from.
  function slide(delta) {
    monthIn.from = (delta > 0 ? 1 : -1) * Style.space(18)
    monthSlide.restart()
  }

  ParallelAnimation {
    id: monthSlide
    NumberAnimation { id: monthIn; target: daysShift; property: "x"; to: 0; duration: 320; easing.type: Easing.OutCubic }
    NumberAnimation { target: days; property: "opacity"; from: 0.2; to: 1; duration: 260; easing.type: Easing.OutCubic }
    NumberAnimation { target: header; property: "opacity"; from: 0.4; to: 1; duration: 260; easing.type: Easing.OutCubic }
  }

  // Keep "today" right across midnight and snap back to it each time the
  // island opens.
  onVisibleChanged: if (visible) goToday()
  Timer {
    interval: 60 * 1000
    repeat: true
    running: root.visible
    onTriggered: root.today = new Date()
  }

  spacing: Style.spacing.md

  Item {
    width: parent.width
    height: Style.space(24)

    Text {
      id: header
      anchors.left: parent.left
      anchors.verticalCenter: parent.verticalCenter
      text: {
        var name = root.locale.standaloneMonthName(root.month, Locale.LongFormat)
        return name.charAt(0).toUpperCase() + name.slice(1) + " " + root.year
      }
      color: root.onCurrentMonth ? root.textColor : Commons.Color.accent
      font.family: Style.font.family
      font.pixelSize: Style.font.title
      font.bold: true
      MouseArea {
        anchors.fill: parent
        cursorShape: root.onCurrentMonth ? Qt.ArrowCursor : Qt.PointingHandCursor
        onClicked: root.goToday()
      }
    }
    Row {
      anchors.right: parent.right
      anchors.verticalCenter: parent.verticalCenter
      IslandButton { glyph: "󰅁"; size: Style.space(24); glyphSize: Style.font.icon; onClicked: root.shift(-1) }
      IslandButton { glyph: "󰅂"; size: Style.space(24); glyphSize: Style.font.icon; onClicked: root.shift(1) }
    }
  }

  Grid {
    id: days
    columns: 7
    transform: Translate { id: daysShift }

    Repeater {
      model: IslandModel.weekdayOrder(root.locale.firstDayOfWeek === 0 ? 7 : root.locale.firstDayOfWeek)
      Text {
        required property int modelData
        width: root.cell
        horizontalAlignment: Text.AlignHCenter
        // Qt's dayName takes 0 = Sunday.
        text: root.locale.dayName(modelData % 7, Locale.NarrowFormat).toUpperCase()
        color: root.dimColor
        font.family: Style.font.family
        font.pixelSize: Style.font.caption
        font.bold: true
      }
    }

    Repeater {
      model: IslandModel.monthGrid(root.year, root.month,
        root.locale.firstDayOfWeek === 0 ? 7 : root.locale.firstDayOfWeek, root.today)
      Item {
        required property var modelData
        width: root.cell
        height: Style.space(24)

        Rectangle {
          anchors.centerIn: parent
          width: Math.min(parent.width, parent.height) - Style.space(2)
          height: width
          radius: width / 2
          color: Commons.Color.accent
          visible: modelData.isToday
        }
        Text {
          anchors.centerIn: parent
          text: modelData.day
          color: modelData.isToday ? Commons.Color.bar.background
            : (!modelData.inMonth ? Util.alpha(Commons.Color.bar.text, 0.25)
              : (modelData.weekend ? root.dimColor : root.textColor))
          font.family: Style.font.family
          font.pixelSize: Style.font.bodySmall
          font.bold: modelData.isToday
        }
      }
    }
  }

  WheelHandler {
    acceptedDevices: PointerDevice.Mouse | PointerDevice.TouchPad
    onWheel: (event) => root.shift(event.angleDelta.y > 0 ? -1 : 1)
  }
}
