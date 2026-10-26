import QtQuick

// A Nerd Font icon sized and positioned by its ink rather than its advance
// box. Icon fonts pad glyphs unevenly, which left play and pause visibly off
// center inside their circles.
Item {
  id: root

  property alias text: label.text
  property alias color: label.color
  property alias font: label.font

  implicitWidth: metrics.tightBoundingRect.width
  implicitHeight: metrics.tightBoundingRect.height

  TextMetrics {
    id: metrics
    font: label.font
    text: label.text
  }

  Text {
    id: label
    // tightBoundingRect is relative to the pen position on the baseline.
    x: -metrics.tightBoundingRect.x
    y: -metrics.tightBoundingRect.y - baselineOffset
  }
}
