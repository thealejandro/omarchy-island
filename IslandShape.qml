import QtQuick
import QtQuick.Shapes

// The island silhouette: a body hanging from the bar with concave "ears" on
// both top corners, so it reads as the bar bulging downwards rather than as
// a separate popup. With a theme cornerRadius of 0 both the ears and the
// bottom corners collapse to square, matching Omarchy's sharp themes.
Shape {
  id: shape

  property real bodyWidth: 0
  property real bodyHeight: 0
  property real radius: 0
  property real ear: radius
  property color fillColor: "black"

  readonly property real e: Math.max(0, Math.min(ear, bodyHeight, bodyWidth / 2))
  readonly property real r: Math.max(0, Math.min(radius, bodyHeight - e, bodyWidth / 2))

  width: bodyWidth + 2 * e
  height: bodyHeight
  visible: bodyWidth > 0.5 && bodyHeight > 0.5
  preferredRendererType: Shape.CurveRenderer

  ShapePath {
    fillColor: shape.fillColor
    strokeWidth: -1
    startX: 0; startY: 0

    PathArc { x: shape.e; y: shape.e; radiusX: shape.e; radiusY: shape.e }
    PathLine { x: shape.e; y: shape.bodyHeight - shape.r }
    PathArc {
      x: shape.e + shape.r; y: shape.bodyHeight
      radiusX: shape.r; radiusY: shape.r
      direction: PathArc.Counterclockwise
    }
    PathLine { x: shape.e + shape.bodyWidth - shape.r; y: shape.bodyHeight }
    PathArc {
      x: shape.e + shape.bodyWidth; y: shape.bodyHeight - shape.r
      radiusX: shape.r; radiusY: shape.r
      direction: PathArc.Counterclockwise
    }
    PathLine { x: shape.e + shape.bodyWidth; y: shape.e }
    PathArc { x: shape.bodyWidth + 2 * shape.e; y: 0; radiusX: shape.e; radiusY: shape.e }
    PathLine { x: 0; y: 0 }
  }
}
