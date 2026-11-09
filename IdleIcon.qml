import QtQuick
import QtQuick.Effects
import QtQuick.Shapes
import "IslandIcons.js" as IslandIcons

// The icon the closed island wears in its notch, and the preview in Settings.
// A built-in glyph (an SVG of the island's own) or a user's file, painted
// through its alpha with `tint` when `tinted` (the same recipe as Omarchy's
// tray: a hidden image as the mask of a coloured rectangle).
//
// The item is the inner box of the notch: `along` px with the bar by `across`
// px deep. On a vertical bar the box is turned (across wide, along tall) and
// a wide glyph is rotated to run along the bar.
Item {
  id: root

  // A bare built-in name ("glasses", "swell"…), or "" to draw `source`.
  property string builtin: ""
  // file:// URL of the user's icon.
  property string source: ""
  property bool tinted: true
  property color tint: "white"
  // The colour of a built-in's small moving part when it has its own (the
  // glasses' dot); the theme's palette has no second hue, so the owner
  // passes the accent here too.
  property color detailTint: tint
  property real along: 36
  property real across: 12
  property bool vertical: false

  // What IslandIcons knows about the built-in: its viewBox and moving part.
  readonly property var glyph: builtin !== "" ? IslandIcons.glyph(builtin) : null
  readonly property bool isIsle: glyph !== null && glyph.isle === true
  readonly property bool hasDetail: IslandIcons.hasDetail(glyph)
  readonly property bool hasImage: !isIsle

  // The image did not decode (or the file vanished): the owner falls back
  // to the plain notch. Not set while there is nothing to draw.
  readonly property bool failed: image.status === Image.Error
  readonly property bool ready: isIsle || image.status === Image.Ready
  readonly property url imageSource: glyph !== null
    ? (glyph.file ? Qt.resolvedUrl("icons/" + glyph.file + ".svg") : "") : source
  readonly property real ratio: glyph !== null ? glyph.w / glyph.h
    : (image.implicitWidth > 0 && image.implicitHeight > 0 ? image.implicitWidth / image.implicitHeight : 1)
  readonly property var placed: IslandIcons.fit(ratio, 1, along, across, vertical)
  // Decoded at the physical size of the box, whatever the file's own size.
  readonly property int decode: Math.max(1, Math.ceil(Math.max(along, across) * Screen.devicePixelRatio))
  // Pixels per unit of the glyph's viewBox.
  readonly property real unit: glyph !== null ? stage.width / glyph.w : 1
  // A square glyph on a vertical bar is not turned, so its dot travels
  // down the bar instead of across it.
  readonly property bool swapAxes: vertical && placed.rotate === 0

  width: vertical ? across : along
  height: vertical ? along : across
  // Once on the whole icon, never on its parts, so overlaps stay solid
  // (the prompt's cursor is the one part that fades, and overlaps nothing).
  opacity: 0.9

  // Where the glance is, 0 to 3: rest, one side, the other, rest.
  property real phase: 0
  // The prompt's cursor.
  property real blink: 1
  readonly property var dotAt: glyph !== null && glyph.dot ? IslandIcons.sample(glyph.dot.keys, phase) : [0, 0]
  // A glyph with nothing of its own moves as a whole, 1.5 px along the bar.
  readonly property real swayPx: hasDetail ? 0 : IslandIcons.sway(phase) * 1.5

  SequentialAnimation {
    id: swing
    NumberAnimation { target: root; property: "phase"; to: 1; duration: 220; easing.type: Easing.OutCubic }
    PauseAnimation { duration: 350 }
    NumberAnimation { target: root; property: "phase"; to: 2; duration: 300; easing.type: Easing.InOutCubic }
    PauseAnimation { duration: 250 }
    NumberAnimation { target: root; property: "phase"; to: 3; duration: 220; easing.type: Easing.OutCubic }
    // Back to rest without a visible jump.
    ScriptAction { script: root.phase = 0 }
  }
  // Like a terminal cursor: off, on, off, on (about 1.2 s), ending visible.
  SequentialAnimation {
    id: blinking
    NumberAnimation { target: root; property: "blink"; to: 0; duration: 120 }
    PauseAnimation { duration: 300 }
    NumberAnimation { target: root; property: "blink"; to: 1; duration: 120 }
    PauseAnimation { duration: 150 }
    NumberAnimation { target: root; property: "blink"; to: 0; duration: 120 }
    PauseAnimation { duration: 300 }
    NumberAnimation { target: root; property: "blink"; to: 1; duration: 120 }
  }
  // Look to one side and back: the glyph's moving part, or the whole glyph.
  function glance() {
    if (swing.running || blinking.running) return
    if (glyph !== null && glyph.blink) blinking.start()
    else swing.start()
  }
  function reset() { swing.stop(); blinking.stop(); phase = 0; blink = 1 }

  Item {
    id: mover
    anchors.fill: parent
    transform: Translate {
      x: !root.vertical ? root.swayPx : 0
      y: root.vertical ? root.swayPx : 0
    }

    Item {
      id: stage
      anchors.centerIn: parent
      width: root.placed.w
      height: root.placed.h
      rotation: root.placed.rotate
      visible: root.ready

      Image {
        id: image
        anchors.fill: parent
        source: root.imageSource
        sourceSize: Qt.size(root.decode, root.decode)
        fillMode: Image.PreserveAspectFit
        asynchronous: true
        cache: false
        smooth: true
        // Kept as a hidden layer so the effect can sample it as a texture.
        visible: !root.tinted
        layer.enabled: root.tinted && root.hasImage
      }
      Rectangle {
        anchors.fill: image
        color: root.tint
        visible: root.tinted && root.hasImage
        layer.enabled: root.tinted && root.hasImage
        layer.effect: MultiEffect {
          maskEnabled: true
          maskSource: image
          maskThresholdMin: 0.5
          maskSpreadAtMin: 1.0
        }
      }
      // A built-in's dot (mark, glasses, capsule, braces).
      Rectangle {
        visible: root.glyph !== null && !!root.glyph.dot
        readonly property real r: root.glyph !== null && root.glyph.dot ? root.glyph.dot.r * root.unit : 0
        width: r * 2
        height: width
        radius: r
        color: root.glyph !== null && root.glyph.dot && root.glyph.dot.detail ? root.detailTint : root.tint
        x: root.dotAt[root.swapAxes ? 1 : 0] * root.unit - r
        y: root.dotAt[root.swapAxes ? 0 : 1] * root.unit - r
      }
      // The prompt's second underscore: the cursor.
      Rectangle {
        visible: root.glyph !== null && !!root.glyph.blink
        readonly property var b: root.glyph !== null && root.glyph.blink ? root.glyph.blink : ({ x: 0, y: 0, w: 0, h: 0, r: 0 })
        x: b.x * root.unit; y: b.y * root.unit
        width: b.w * root.unit; height: b.h * root.unit
        radius: b.r * root.unit
        color: root.tint
        opacity: root.blink
      }
      // The isle: a mound that leans and a wave that shifts, as paths.
      Shape {
        visible: root.isIsle
        anchors.fill: parent
        preferredRendererType: Shape.CurveRenderer
        ShapePath {
          fillColor: root.tint
          strokeColor: "transparent"
          strokeWidth: 0
          PathSvg { path: root.isIsle ? IslandIcons.isleMound(root.phase, root.unit) : "" }
        }
        ShapePath {
          fillColor: "transparent"
          strokeColor: root.tint
          strokeWidth: 2 * root.unit
          capStyle: ShapePath.RoundCap
          joinStyle: ShapePath.RoundJoin
          PathSvg { path: root.isIsle ? IslandIcons.isleWave(root.phase, root.unit) : "" }
        }
      }
    }
  }
}
