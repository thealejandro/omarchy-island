import QtQuick
import qs.Commons
import qs.Commons as Commons
import "IslandModel.js" as IslandModel

// The world page: the weather here with the next three days, then the
// cities from elsewhen (or the island's own list) with their time, day and
// difference from here.
Column {
  id: root

  required property var island

  readonly property color textColor: Commons.Color.bar.text
  readonly property color dimColor: Util.alpha(Commons.Color.bar.text, 0.72)
  readonly property color lineColor: Util.alpha(Commons.Color.bar.text, 0.2)
  readonly property var weather: island.weather

  function tr(key, arg) { return island.tr(key, arg) }

  function dayName(iso, index) {
    if (index === 0) return tr("today")
    var parts = String(iso).split("-")
    var d = new Date(parseInt(parts[0], 10), parseInt(parts[1], 10) - 1, parseInt(parts[2], 10))
    return island.locale.toString(d, "ddd")
  }

  function dayLabel(delta) {
    if (delta === 0) return ""
    return tr(delta > 0 ? "tomorrow" : "yesterday")
  }

  spacing: Style.spacing.lg

  component Pill: Rectangle {
    id: pill
    property string label: ""
    signal clicked()
    width: pillText.implicitWidth + 2 * Style.spacing.xl
    height: Style.space(26)
    radius: Style.space(8)
    color: Util.alpha(root.textColor, pillMouse.containsMouse ? 0.13 : 0.07)
    scale: pillMouse.pressed ? 0.95 : 1
    Behavior on color { ColorAnimation { duration: 120 } }
    Behavior on scale { NumberAnimation { duration: 140 } }
    Text {
      id: pillText
      anchors.centerIn: parent
      text: pill.label
      color: root.textColor
      font.family: Style.font.family
      font.pixelSize: Style.font.caption
      font.bold: true
    }
    MouseArea {
      id: pillMouse
      anchors.fill: parent
      hoverEnabled: true
      cursorShape: Qt.PointingHandCursor
      onClicked: pill.clicked()
    }
  }

  // ---------------------------------------------------------------- weather

  Row {
    width: parent.width
    spacing: Style.spacing.xl
    visible: !!root.weather

    Glyph {
      anchors.verticalCenter: parent.verticalCenter
      text: root.island.weatherGlyph
      color: root.textColor
      font.family: Style.font.family
      font.pixelSize: Style.space(30)
    }
    Column {
      anchors.verticalCenter: parent.verticalCenter
      width: parent.width - Style.space(40) - (root.island.cfg.forecast ? forecast.width : 0) - 2 * parent.spacing
      Text {
        width: parent.width
        elide: Text.ElideRight
        text: root.weather ? root.weather.temp + "  " + root.weather.desc : ""
        color: root.textColor
        font.family: Style.font.family
        font.pixelSize: Style.font.body
        font.bold: true
      }
      Text {
        width: parent.width
        elide: Text.ElideRight
        text: root.weather ? root.weather.place : ""
        visible: text !== ""
        color: root.dimColor
        font.family: Style.font.family
        font.pixelSize: Style.font.caption
      }
    }

    Row {
      id: forecast
      anchors.verticalCenter: parent.verticalCenter
      spacing: Style.spacing.lg
      visible: root.island.cfg.forecast && !!root.weather && root.weather.days.length > 0
      Repeater {
        model: root.weather && root.island.cfg.forecast ? root.weather.days : []
        Column {
          id: day
          required property var modelData
          required property int index
          width: Style.space(52)
          spacing: Style.spacing.xs
          Text {
            anchors.horizontalCenter: parent.horizontalCenter
            text: root.dayName(day.modelData.date, day.index)
            color: root.dimColor
            font.family: Style.font.family
            font.pixelSize: Style.font.caption
            font.capitalization: Font.Capitalize
          }
          Glyph {
            anchors.horizontalCenter: parent.horizontalCenter
            text: IslandModel.weatherIcon(day.modelData.code, false)
            color: root.textColor
            font.family: Style.font.family
            font.pixelSize: Style.font.body + Style.space(4)
          }
          Text {
            anchors.horizontalCenter: parent.horizontalCenter
            text: day.modelData.max + " " + day.modelData.min
            color: root.textColor
            font.family: Style.font.family
            font.pixelSize: Style.font.caption
            font.features: { "tnum": 1 }
          }
        }
      }
    }
  }

  Flow {
    width: parent.width
    spacing: Style.spacing.sm
    Pill {
      visible: root.island.weatherOnBar
      label: root.tr("act_forecast")
      onClicked: root.island.openForecast()
    }
    Pill { label: root.tr("act_weatherPlace"); onClicked: root.island.changeWeatherPlace() }
  }

  Rectangle {
    width: parent.width
    height: 1
    color: root.lineColor
    visible: root.island.worldRows.length > 0
  }

  // ---------------------------------------------------------------- cities

  Column {
    width: parent.width
    spacing: Style.spacing.sm
    visible: root.island.worldRows.length > 0

    Repeater {
      model: root.island.worldRows
      Item {
        id: city
        required property var modelData
        width: parent.width
        height: Style.space(30)

        Glyph {
          id: sun
          anchors.left: parent.left
          anchors.verticalCenter: parent.verticalCenter
          width: Style.space(22)
          text: city.modelData.night ? "󰖔" : "󰖙"
          color: root.dimColor
          font.family: Style.font.family
          font.pixelSize: Style.font.body
        }
        Column {
          anchors.left: sun.right
          anchors.right: clock.left
          anchors.rightMargin: Style.spacing.lg
          anchors.verticalCenter: parent.verticalCenter
          Text {
            width: parent.width
            elide: Text.ElideRight
            text: city.modelData.name
            color: root.textColor
            font.family: Style.font.family
            font.pixelSize: Style.font.bodySmall
            font.bold: true
          }
          Text {
            width: parent.width
            elide: Text.ElideRight
            text: [city.modelData.abbr, IslandModel.diffLabel(city.modelData.diffMinutes), root.dayLabel(city.modelData.dayDelta)]
              .filter(function(s) { return s !== "" }).join(" · ")
            color: root.dimColor
            font.family: Style.font.family
            font.pixelSize: Style.font.caption
          }
        }
        Text {
          id: clock
          anchors.right: parent.right
          anchors.verticalCenter: parent.verticalCenter
          text: city.modelData.time + (city.modelData.meridiem ? " " + city.modelData.meridiem : "")
          color: root.textColor
          font.family: Style.font.family
          font.pixelSize: Style.font.body + Style.space(2)
          font.bold: true
          font.features: { "tnum": 1 }
        }
      }
    }
  }

  Text {
    width: parent.width
    visible: root.island.worldRows.length === 0
    wrapMode: Text.WordWrap
    text: root.tr("world_empty")
    color: root.dimColor
    font.family: Style.font.family
    font.pixelSize: Style.font.caption
  }
}
