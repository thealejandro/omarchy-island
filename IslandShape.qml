import QtQuick
import QtQuick.Shapes
import "IslandGeometry.js" as IslandGeometry

// The island silhouette: a body hanging from the bar with concave "ears" on
// both top corners, so it reads as the bar bulging downwards rather than as
// a separate popup. With a theme cornerRadius of 0 both the ears and the
// bottom corners collapse to square, matching Omarchy's sharp themes.
//
// It is always drawn hanging from a top bar (bodyWidth runs along the bar,
// bodyHeight is the depth) and then turned for the bar's `edge` with one
// matrix, so the four orientations share a single path. Place it at (0, 0)
// of the body: the matrix already pushes the ears out past both ends.
Shape {
  id: shape

  property string edge: "top"

  property real bodyWidth: 0
  property real bodyHeight: 0
  property real radius: 0
  property real ear: radius
  property color fillColor: "black"
  // Colour the body fades to below `fadeLength` px. Matches fillColor unless
  // the bar is glassy: the island starts with the bar's frosted tint where they
  // meet and turns more solid further down, so content behind it can't show
  // through the text.
  property color fillColorEnd: fillColor
  property real fadeLength: 40
  // Outline along the sides and bottom only: the top edge stays open so the
  // body still reads as part of the bar. Without it the island disappears on
  // light themes, where the bar colour is close to the wallpaper.
  property color strokeColor: "transparent"
  property real strokeWidth: 0

  readonly property real e: Math.max(0, Math.min(ear, bodyHeight, bodyWidth / 2))
  readonly property real r: Math.max(0, Math.min(radius, bodyHeight - e, bodyWidth / 2))

  width: bodyWidth + 2 * e
  height: bodyHeight
  visible: bodyWidth > 0.5 && bodyHeight > 0.5
  preferredRendererType: Shape.CurveRenderer

  transform: Matrix4x4 {
    matrix: {
      var m = IslandGeometry.shapeMatrix(shape.edge, shape.bodyHeight, shape.e)
      return Qt.matrix4x4(m[0], m[1], m[2], m[3], m[4], m[5], m[6], m[7],
        m[8], m[9], m[10], m[11], m[12], m[13], m[14], m[15])
    }
  }

  ShapePath {
    fillGradient: LinearGradient {
      x1: 0; y1: 0
      x2: 0; y2: Math.max(1, Math.min(shape.fadeLength, shape.bodyHeight))
      GradientStop { position: 0; color: shape.fillColor }
      GradientStop { position: 1; color: shape.fillColorEnd }
    }
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

  ShapePath {
    fillColor: "transparent"
    strokeColor: shape.strokeColor
    strokeWidth: shape.strokeWidth > 0 ? shape.strokeWidth : -1
    capStyle: ShapePath.FlatCap
    joinStyle: ShapePath.RoundJoin
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
  }
}
