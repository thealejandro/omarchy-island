.pragma library

// The icon the closed island can wear in its notch (idleNotch "icon"): eleven
// built-in glyphs and the user's own .svg or .png files from
// ~/.config/omarchy/island/icons. Pure logic only; the files are read in
// Service.qml and drawn by IdleIcon.qml.

// Bare names. Anything else has to be a file name with an extension.
var BUILTINS = ["glasses", "isle", "prompt", "capsule", "braces", "mark", "archipelago", "swell", "channel", "fold", "branch"]

// The glance runs one phase from 0 to 3 through four keyframes: rest, A, B,
// rest. Everything that moves is a list of four keyframes and sample().
var PHASE_END = 3

// keyframes for a dot that goes rest -> a -> b -> rest, in the glyph's own units
function track(rest, a, b) { return [rest, a, b, rest] }

// The built-ins. `file`: the static SVG (painted through its alpha), absent
// when the glyph is drawn entirely in QML. w x h: the SVG's viewBox, in whose
// units everything below is given. Moving parts, all optional:
//   dot:   { r, keys, detail }  a circle following keys (x,y per keyframe);
//          `detail` paints it with the detail colour instead of the tint
//   blink: { x, y, w, h, r }    a rect that blinks on the glance
//   isle:  true                 mound and wave drawn in QML (isleMound/isleWave)
// A glyph with none of them sways as a whole instead (like a user's icon).
var GLYPHS = {
  glasses: { file: "glasses", w: 32, h: 12, dot: { r: 2.2, keys: track([8, 6], [10.5, 6], [24, 6]), detail: true } },
  isle: { w: 24, h: 16, isle: true },
  prompt: { file: "prompt", w: 32, h: 12, blink: { x: 20.5, y: 8.5, w: 6.5, h: 2, r: 1 } },
  capsule: { file: "capsule", w: 32, h: 12, dot: { r: 2.2, keys: track([16, 6], [24.5, 6], [7.5, 6]) } },
  braces: { file: "braces", w: 32, h: 12, dot: { r: 2, keys: track([16, 6], [20.5, 6], [11.5, 6]) } },
  mark: { file: "mark-ring", w: 24, h: 24, dot: { r: 3.5, keys: track([12, 12], [15, 12], [9, 12]) } },
  archipelago: { file: "archipelago", w: 24, h: 24 },
  swell: { file: "swell", w: 24, h: 24 },
  channel: { file: "channel", w: 32, h: 12 },
  fold: { file: "fold", w: 32, h: 12 },
  branch: { file: "branch", w: 32, h: 12 }
}

function glyph(name) {
  return typeof name === "string" && Object.prototype.hasOwnProperty.call(GLYPHS, name) ? GLYPHS[name] : null
}

// Whether the glyph has a part of its own that moves (else it sways).
function hasDetail(g) { return !!(g && (g.dot || g.blink || g.isle)) }

// The value at `phase` on a keyframe list (arrays of numbers, all the same
// length), linear between neighbours; phase is clamped to the list.
function sample(keys, phase) {
  var n = keys.length - 1
  var p = Math.max(0, Math.min(n, Number(phase) || 0))
  var i = Math.min(n - 1, Math.floor(p))
  var f = p - i
  var a = keys[i], b = keys[i + 1], out = []
  for (var k = 0; k < a.length; k++) out.push(a[k] + (b[k] - a[k]) * f)
  return out
}

// Whole-glyph sway, in units of the amplitude: 0 -> +1 -> -1 -> 0.
var SWAY = [[0], [1], [-1], [0]]
function sway(phase) { return sample(SWAY, phase)[0] }

// The isle's mound (24 x 16 units): control points of its two curves after
// M4 10.5, at rest and leaning right and left.
var ISLE_REST = [5.5, 5.5, 9, 3.5, 12, 3.5, 15, 3.5, 18.5, 5.5, 20, 10.5]
var ISLE_RIGHT = [7, 7, 10.5, 3.5, 14, 3.5, 16, 3.5, 19.5, 6, 20, 10.5]
var ISLE_LEFT = [4.5, 6, 8, 3.5, 10, 3.5, 13.5, 3.5, 17, 7, 20, 10.5]
var ISLE_KEYS = [ISLE_REST, ISLE_RIGHT, ISLE_LEFT, ISLE_REST]

function num(v, scale) { return Math.round(v * scale * 1000) / 1000 }

// SVG path data of the mound at `phase`, scaled by `scale` px per unit.
function isleMound(phase, scale) {
  var c = sample(ISLE_KEYS, phase), s = scale
  function pt(i) { return num(c[i], s) + " " + num(c[i + 1], s) }
  return "M " + num(4, s) + " " + num(10.5, s)
    + " C " + pt(0) + " " + pt(2) + " " + pt(4)
    + " C " + pt(6) + " " + pt(8) + " " + pt(10) + " Z"
}

// The wave under it (stroked, width 2 units), shifted up to 1 unit along x.
function isleWave(phase, scale) {
  var s = scale, t = " t " + num(4.5, s) + " 0"
  return "M " + num(3 + sway(phase), s) + " " + num(13.5, s)
    + " q " + num(2.25, s) + " " + num(-2, s) + " " + num(4.5, s) + " 0" + t + t + t
}

var MAX_BYTES = 262144
var MAX_PIXELS = 1024
var MAX_LISTED = 64

// A bare file name, never a path: no separators, no leading dot, no "..",
// no control characters. Returns the name, or "" when it is not safe.
function safeName(value) {
  if (typeof value !== "string") return ""
  if (!/^[A-Za-z0-9_][A-Za-z0-9_. -]{0,62}\.(svg|png)$/i.test(value)) return ""
  if (value.indexOf("..") !== -1) return ""
  return value
}

// What a setting value names: { builtin }, { file } or null.
function resolve(value) {
  if (typeof value !== "string") return null
  if (BUILTINS.indexOf(value) !== -1) return { builtin: value }
  var name = safeName(value)
  return name ? { file: name } : null
}

// The output of `find … -printf '%f\t%s\n'`: safe names of non-empty files up
// to 256 KiB, once each, sorted without regard to case, at most 64.
function parseListing(text) {
  var seen = {}, out = []
  var lines = String(text || "").split("\n")
  for (var i = 0; i < lines.length; i++) {
    var bits = lines[i].split("\t")
    if (bits.length !== 2) continue
    var name = safeName(bits[0])
    var size = Number(bits[1])
    if (!name || !isFinite(size) || size <= 0 || size > MAX_BYTES) continue
    if (seen[name]) continue
    seen[name] = true
    out.push(name)
  }
  out.sort(function(a, b) {
    var x = a.toLowerCase(), y = b.toLowerCase()
    return x < y ? -1 : (x > y ? 1 : (a < b ? -1 : (a > b ? 1 : 0)))
  })
  return out.slice(0, MAX_LISTED)
}

// The picker's choices in order: the built-ins, the user's files, and the
// current value last when it is neither (a file that is gone, or not listed yet).
function pickerList(files, current) {
  var out = BUILTINS.concat(Array.isArray(files) ? files : [])
  if (typeof current === "string" && current !== "" && out.indexOf(current) === -1) out.push(current)
  return out
}

// A PNG's signature and IHDR from its first bytes (any array-like of numbers).
function inspectPng(bytes) {
  var bad = { ok: false, width: 0, height: 0 }
  if (!bytes || typeof bytes.length !== "number" || bytes.length < 24) return bad
  var sig = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]
  for (var i = 0; i < 8; i++) if (bytes[i] !== sig[i]) return bad
  // First chunk: length 13, type "IHDR".
  var len = ((bytes[8] << 24) | (bytes[9] << 16) | (bytes[10] << 8) | bytes[11]) >>> 0
  if (len !== 13 || bytes[12] !== 0x49 || bytes[13] !== 0x48 || bytes[14] !== 0x44 || bytes[15] !== 0x52) return bad
  var w = ((bytes[16] << 24) | (bytes[17] << 16) | (bytes[18] << 8) | bytes[19]) >>> 0
  var h = ((bytes[20] << 24) | (bytes[21] << 16) | (bytes[22] << 8) | bytes[23]) >>> 0
  if (w < 1 || h < 1 || w > MAX_PIXELS || h > MAX_PIXELS) return bad
  return { ok: true, width: w, height: h }
}

function normalizeColor(c) {
  var s = String(c).toLowerCase()
  if (/^#[0-9a-f]{3}$/.test(s)) s = "#" + s[1] + s[1] + s[2] + s[2] + s[3] + s[3]
  return s
}

// An SVG is acceptable when it has an <svg root and neither scripts nor
// foreign objects. `mono`: it paints with at most one colour, so it can be
// tinted like a symbolic icon. Gradients, patterns and filters (any url()) are
// never mono. An element with no fill at all paints black and is not counted.
function inspectSvg(text) {
  var bad = { ok: false, mono: false }
  if (typeof text !== "string" || text.length === 0 || text.length > MAX_BYTES) return bad
  if (!/<svg[\s>]/i.test(text)) return bad
  if (/<script/i.test(text) || /<foreignObject/i.test(text)) return bad
  if (/url\(/i.test(text)) return { ok: true, mono: false }
  var colors = {}, count = 0
  var re = /\b(?:fill|stroke)\s*[=:]\s*["']?\s*([^"';}\s>]+)/gi
  var m
  while ((m = re.exec(text)) !== null) {
    var v = m[1].toLowerCase()
    if (v === "none" || v === "transparent" || v === "currentcolor" || v === "inherit") continue
    v = normalizeColor(v)
    if (!colors[v]) { colors[v] = true; count++ }
  }
  return { ok: true, mono: count <= 1 }
}

// Whether the glyph is painted with the accent through its alpha. `policy` is
// "auto" | "on" | "off"; `res` comes from resolve(); `mono` from inspectSvg.
// Built-ins are always tinted; a PNG is tinted by auto only when it is
// "-symbolic".
function tinted(policy, res, mono) {
  if (!res) return false
  if (res.builtin) return true
  if (policy === "on") return true
  if (policy === "off") return false
  var name = String(res.file || "")
  if (/-symbolic\.(svg|png)$/i.test(name)) return true
  return /\.svg$/i.test(name) && mono === true
}

// How a glyph of natural size natW x natH sits in the notch's inner box:
// `along` (with the bar) by `across` (its depth). { w, h } is the item's own
// size, preserving the aspect; `rotate` is 90 when a wide glyph on a vertical
// bar is turned to run along it. Square or tall glyphs stay upright.
function fit(natW, natH, along, across, vertical) {
  var nw = Number(natW) > 0 ? Number(natW) : 1
  var nh = Number(natH) > 0 ? Number(natH) : 1
  var a = Math.max(0, Number(along) || 0), c = Math.max(0, Number(across) || 0)
  var rotate = vertical && nw > nh ? 90 : 0
  // The item's own axes: rotated, its width runs along the bar.
  var upright = vertical && rotate === 0
  var boxW = upright ? c : a
  var boxH = upright ? a : c
  var s = Math.min(boxW / nw, boxH / nh)
  return { w: nw * s, h: nh * s, rotate: rotate }
}
