.pragma library

// Where the island sits, for a bar on any edge. Everything is worked out in
// two axes: "along" runs with the bar (x on a top or bottom bar, y on a side
// bar) and "across" is the depth from the screen edge into the screen. The
// island's window spans the whole length of its edge, so along coordinates
// are the same as the screen's.

var EDGES = ["top", "bottom", "left", "right"]

function edge(position) {
  var p = String(position || "").toLowerCase()
  return EDGES.indexOf(p) !== -1 ? p : "top"
}

function isVertical(e) {
  return e === "left" || e === "right"
}

// A rectangle in window coordinates from along/across extents. winW and
// winH are the island window's size; across is measured from the edge the
// bar is on, so "from 0" always touches the screen edge.
function rect(e, winW, winH, alongStart, alongLen, acrossFrom, acrossLen) {
  var a = Math.round(alongStart), l = Math.max(0, Math.round(alongLen))
  var f = Math.round(acrossFrom), d = Math.max(0, Math.round(acrossLen))
  switch (e) {
    case "bottom": return { x: a, y: winH - f - d, width: l, height: d }
    case "left": return { x: f, y: a, width: d, height: l }
    case "right": return { x: winW - f - d, y: a, width: d, height: l }
    default: return { x: a, y: f, width: l, height: d }
  }
}

// Keeps a span of `len` centered on `center` inside [margin, total - margin],
// so an island anchored to a clock near a screen corner stays on screen.
function clampCenter(center, len, total, margin) {
  var half = len / 2
  var lo = half + margin, hi = total - half - margin
  if (hi < lo) return total / 2
  return Math.max(lo, Math.min(hi, center))
}

// The 4x4 matrix (row-major, as Qt.matrix4x4 takes it) that turns the
// island silhouette, drawn hanging from a top bar with its ears at y = 0,
// into the right orientation for edge `e`. u runs along the silhouette
// (0..along + 2 * ear), v is its depth (0..depth). The result sits in the
// body's own coordinates, ears sticking out by `ear` on both ends.
function shapeMatrix(e, depth, ear) {
  switch (e) {
    case "bottom": return [1, 0, 0, -ear, 0, -1, 0, depth, 0, 0, 1, 0, 0, 0, 0, 1]
    case "left": return [0, 1, 0, 0, 1, 0, 0, -ear, 0, 0, 1, 0, 0, 0, 0, 1]
    case "right": return [0, -1, 0, depth, 1, 0, 0, -ear, 0, 0, 1, 0, 0, 0, 0, 1]
    default: return [1, 0, 0, -ear, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]
  }
}

// Breadth-first search below `rootItem` for the bar slot or widget of
// `moduleName`. Best effort over the bar's internals: anything unexpected
// just means "not found". Visits at most `limit` items.
function findModule(rootItem, moduleName, limit) {
  if (!rootItem) return null
  var queue = [rootItem], seen = 0, max = limit || 4000
  while (queue.length > 0 && seen < max) {
    var item = queue.shift()
    seen++
    try {
      if (item !== rootItem && item.moduleName === moduleName && item.width > 0 && item.height > 0
          && item.visible !== false)
        return item
      var kids = item.children
      if (kids) for (var i = 0; i < kids.length; i++) if (kids[i]) queue.push(kids[i])
    } catch (e) {
      // A child torn down mid-walk; skip it.
    }
  }
  return null
}

// Along-axis center and length of `item` in its window's coordinates.
function alongSpan(item, vertical) {
  var p = item.mapToItem(null, 0, 0)
  return vertical
    ? { center: p.y + item.height / 2, length: item.height }
    : { center: p.x + item.width / 2, length: item.width }
}
