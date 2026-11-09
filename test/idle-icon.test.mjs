// The notch icon (idleNotch "icon"): file names, listing, PNG and SVG checks,
// tint policy, fitting, look, motion and the setting.
import { test } from "node:test"
import assert from "node:assert/strict"
import { existsSync } from "node:fs"
import { load, plain, readJson } from "./load.mjs"

const Ic = load("IslandIcons.js")
const II = load("IslandIdle.js")
const IS = load("IslandSettings.js")
const IM = load("IslandModel.js")

test("safeName: bare .svg or .png names only", () => {
  for (const ok of ["rocket.svg", "My Icon-symbolic.png", "A_b-c.1.SVG", "9.png"])
    assert.equal(Ic.safeName(ok), ok, ok)
  for (const bad of ["../x.svg", "/etc/x.svg", "a/b.svg", "a\\b.svg", ".hidden.svg", "x.jpg", "x.svg\n", "x\u0000.svg",
    "a..b.svg", "x".repeat(80) + ".svg", "", ".svg", "mark", " x.svg", "x.svg ", "x.svgz"])
    assert.equal(Ic.safeName(bad), "", JSON.stringify(bad))
  for (const bad of [null, undefined, 3, {}, ["a.svg"], true]) assert.equal(Ic.safeName(bad), "")
  // 63 characters before the extension is the longest.
  assert.equal(Ic.safeName("x".repeat(63) + ".svg"), "x".repeat(63) + ".svg")
  assert.equal(Ic.safeName("x".repeat(64) + ".svg"), "")
})

test("resolve: bare names are built-ins, files carry an extension", () => {
  assert.deepEqual(plain(Ic.BUILTINS), ["glasses", "isle", "prompt", "capsule", "braces", "mark", "archipelago", "swell", "channel", "fold"])
  for (const b of Ic.BUILTINS) assert.deepEqual(plain(Ic.resolve(b)), { builtin: b })
  assert.deepEqual(plain(Ic.resolve("mark.svg")), { file: "mark.svg" })
  assert.deepEqual(plain(Ic.resolve("rocket.png")), { file: "rocket.png" })
  for (const bad of ["Mark", "builtin:mark", "../mark", "", "rocket", null, 4, "a/b.svg"])
    assert.equal(Ic.resolve(bad), null, String(bad))
})

test("every built-in has its SVG file", () => {
  for (const b of Ic.BUILTINS) {
    const g = Ic.glyph(b)
    assert.ok(g, b)
    // The isle is drawn entirely in QML.
    if (g.file) assert.ok(existsSync(new URL(`../icons/${g.file}.svg`, import.meta.url)), g.file)
    else assert.equal(b, "isle")
  }
  assert.equal(Ic.glyph("nope"), null)
  assert.equal(Ic.glyph("constructor"), null)
  assert.equal(Ic.glyph(null), null)
})

test("parseListing: safe names, sane sizes, unique, sorted, capped", () => {
  const text = [
    "b.svg\t100", "A.png\t200", "B.SVG\t5", "b.svg\t100", "empty.svg\t0", "huge.png\t262145",
    "edge.png\t262144", "../x.svg\t10", "bad.jpg\t10", ".h.svg\t10", "nosize.svg", "x.svg\tabc", "", "a\tb\tc",
  ].join("\n")
  assert.deepEqual(plain(Ic.parseListing(text)), ["A.png", "B.SVG", "b.svg", "edge.png"])
  assert.deepEqual(plain(Ic.parseListing("")), [])
  assert.deepEqual(plain(Ic.parseListing(null)), [])
  const many = Array.from({ length: 100 }, (_, i) => `i${String(i).padStart(3, "0")}.svg\t10`).join("\n")
  const out = plain(Ic.parseListing(many))
  assert.equal(out.length, 64)
  assert.equal(out[0], "i000.svg")
})

test("pickerList: built-ins, files, then an unlisted current value", () => {
  assert.deepEqual(plain(Ic.pickerList(["a.svg"], "glasses")), [...Ic.BUILTINS, "a.svg"])
  assert.deepEqual(plain(Ic.pickerList(["a.svg"], "a.svg")), [...Ic.BUILTINS, "a.svg"])
  assert.deepEqual(plain(Ic.pickerList(["a.svg"], "gone.png")), [...Ic.BUILTINS, "a.svg", "gone.png"])
  assert.deepEqual(plain(Ic.pickerList(null, "")), plain(Ic.BUILTINS))
})

function png(w, h, extra = {}) {
  const b = new Uint8Array(33)
  b.set([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a], 0)
  b.set([0, 0, 0, 13], 8)
  b.set([0x49, 0x48, 0x44, 0x52], 12)
  new DataView(b.buffer).setUint32(16, w)
  new DataView(b.buffer).setUint32(20, h)
  if (extra.sig) b[1] = 0x51
  if (extra.type) b[12] = 0x4a
  return extra.cut ? b.slice(0, extra.cut) : b
}

test("inspectPng: signature, IHDR and size", () => {
  assert.deepEqual(plain(Ic.inspectPng(png(24, 16))), { ok: true, width: 24, height: 16 })
  assert.equal(Ic.inspectPng(png(1024, 1024)).ok, true)
  assert.equal(Ic.inspectPng(png(1025, 10)).ok, false)
  assert.equal(Ic.inspectPng(png(10, 1025)).ok, false)
  assert.equal(Ic.inspectPng(png(0, 10)).ok, false)
  assert.equal(Ic.inspectPng(png(4294967295, 4294967295)).ok, false)
  assert.equal(Ic.inspectPng(png(10, 10, { sig: true })).ok, false)
  assert.equal(Ic.inspectPng(png(10, 10, { type: true })).ok, false)
  assert.equal(Ic.inspectPng(png(10, 10, { cut: 23 })).ok, false)
  assert.equal(Ic.inspectPng(png(10, 10, { cut: 24 })).ok, true)
  for (const bad of [null, undefined, [], "x", 5]) assert.equal(Ic.inspectPng(bad).ok, false)
})

const svg = (body, attrs = "") => `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"${attrs}>${body}</svg>`

test("inspectSvg: accepts SVG, counts colours", () => {
  const mono = [
    svg('<path d="M0 0h1"/>'),
    svg('<path fill="#000" d="M0 0h1"/>'),
    svg('<path fill="#000"/><path fill="#000000"/>'.replace("#000000", "#000")),
    svg('<path fill="#fff"/><path fill="#FFF"/>'),
    svg('<path fill="none" stroke="#123456" stroke-width="2"/>'),
    svg('<path style="fill:#abc;stroke:none"/>'),
    svg('<path fill="currentColor"/><path fill="inherit"/><path fill="transparent" stroke="red"/>'),
    svg('<path fill="red" fill-rule="evenodd" fill-opacity="0.5"/>'),
  ]
  for (const t of mono) assert.deepEqual(plain(Ic.inspectSvg(t)), { ok: true, mono: true }, t)
  const multi = [
    svg('<path fill="#000"/><path fill="#f00"/>'),
    svg('<path fill="red" stroke="blue"/>'),
    svg('<path style="fill:#abc"/><path fill="#def"/>'),
    svg('<defs><linearGradient id="a"/></defs><path fill="url(#a)"/>'),
    svg('<path fill="#000" filter="url(#f)"/>'),
  ]
  for (const t of multi) assert.deepEqual(plain(Ic.inspectSvg(t)), { ok: true, mono: false }, t)
})

test("inspectSvg: rejects scripts, foreign objects and non-SVG", () => {
  for (const t of [svg("<script>alert(1)</script>"), svg("<SCRIPT src=x></SCRIPT>"), svg("<foreignObject/>"),
    svg("<foreignobject/>"), "<html></html>", "", "PNG", null, undefined, 5, "x".repeat(262145)])
    assert.equal(Ic.inspectSvg(t).ok, false, String(t).slice(0, 40))
})

test("tinted: built-ins always, user files by policy", () => {
  const b = Ic.resolve("mark"), f = (n) => Ic.resolve(n)
  for (const p of ["auto", "on", "off"]) assert.equal(Ic.tinted(p, b, false), true)
  assert.equal(Ic.tinted("auto", f("a.svg"), true), true)
  assert.equal(Ic.tinted("auto", f("a.svg"), false), false)
  assert.equal(Ic.tinted("auto", f("a.png"), true), false)
  assert.equal(Ic.tinted("auto", f("a-symbolic.png"), false), true)
  assert.equal(Ic.tinted("auto", f("a-symbolic.SVG"), false), true)
  assert.equal(Ic.tinted("on", f("a.png"), false), true)
  assert.equal(Ic.tinted("on", f("a.svg"), false), true)
  assert.equal(Ic.tinted("off", f("a-symbolic.svg"), true), false)
  assert.equal(Ic.tinted("off", f("a.svg"), true), false)
  assert.equal(Ic.tinted("auto", null, true), false)
})

test("fit: aspect kept; wide glyphs turn on vertical bars", () => {
  const near = (a, b) => assert.ok(Math.abs(a - b) < 1e-9, `${a} vs ${b}`)
  // Horizontal: 32:12 in 24 x 12 is limited by the width.
  let r = Ic.fit(32, 12, 24, 12, false)
  near(r.w, 24); near(r.h, 9); assert.equal(r.rotate, 0)
  r = Ic.fit(1, 1, 24, 12, false)
  near(r.w, 12); near(r.h, 12); assert.equal(r.rotate, 0)
  // Vertical: a wide glyph is turned and keeps running along the bar.
  r = Ic.fit(32, 12, 24, 12, true)
  near(r.w, 24); near(r.h, 9); assert.equal(r.rotate, 90)
  // Vertical: a square glyph stays upright (across wide, along tall).
  r = Ic.fit(1, 1, 24, 12, true)
  near(r.w, 12); near(r.h, 12); assert.equal(r.rotate, 0)
  // A tall glyph stays upright and fits across x along.
  r = Ic.fit(12, 32, 24, 12, true)
  near(r.h, 24); near(r.w, 9); assert.equal(r.rotate, 0)
  // Junk sizes behave as square.
  r = Ic.fit(0, NaN, 24, 12, false)
  near(r.w, 12); near(r.h, 12)
})

test("look: icon needs an icon, never yields to busy", () => {
  assert.equal(II.look("icon", "closed", false, false, false, true), "icon")
  assert.equal(II.look("icon", "closed", false, true, false, true), "icon")
  assert.equal(II.look("icon", "closed", false, false, false, false), "notch")
  assert.equal(II.look("icon", "closed", false, false, false), "notch")
  assert.equal(II.look("icon", "open", false, false, false, true), "none")
  assert.equal(II.look("icon", "peek", false, false, false, true), "none")
  assert.equal(II.look("icon", "closed", true, false, false, true), "none")
  // The other looks ignore the new argument.
  assert.equal(II.look("hour", "closed", false, false, false, true), "hour")
  assert.equal(II.look("off", "closed", false, false, false, true), "none")
})

test("glanceInterval: 20 to 45 s", () => {
  let lo = Infinity, hi = -Infinity
  for (let i = 0; i < 1000; i++) {
    const v = II.glanceInterval(Math.random())
    lo = Math.min(lo, v); hi = Math.max(hi, v)
    assert.ok(v >= 20000 && v <= 45000, String(v))
  }
  assert.equal(II.glanceInterval(0), 20000)
  assert.equal(II.glanceInterval(1), 45000)
  assert.equal(II.glanceInterval(-3), 20000)
  assert.equal(II.glanceInterval(9), 45000)
  assert.equal(II.glanceInterval(NaN), 20000)
  assert.equal(II.glanceInterval(undefined), 20000)
})

test("motionActive: closed, visible, icon, motion and animations on", () => {
  const base = { motion: true, animations: true, look: "icon", hasIcon: true, mode: "closed", barHidden: false }
  assert.equal(II.motionActive(base), true)
  for (const over of [{ motion: false }, { animations: false }, { look: "notch" }, { look: "hour" }, { hasIcon: false },
    { mode: "open" }, { mode: "peek" }, { barHidden: true }])
    assert.equal(II.motionActive({ ...base, ...over }), false, JSON.stringify(over))
  assert.equal(II.motionActive(null), false)
  assert.equal(II.motionActive({}), false)
})

test("canNudge: at least 3 s after the last glance", () => {
  assert.equal(II.canNudge(0, 1000), true)
  assert.equal(II.canNudge(10000, 12999), false)
  assert.equal(II.canNudge(10000, 13000), true)
  assert.equal(II.canNudge(10000, 5000), true)
  assert.equal(II.canNudge(NaN, 5000), true)
  assert.equal(II.canNudge(10000, NaN), false)
})

test("settings: idleNotch gains icon; idleIcon, idleIconTint and idleMotion", () => {
  assert.deepEqual(plain(IS.spec("idleNotch").options), ["off", "notch", "icon", "hour", "day", "battery"])
  assert.equal(IS.spec("idleIcon").def, "glasses")
  assert.equal(IS.spec("idleIcon").type, "string")
  assert.deepEqual(plain(IS.spec("idleIconTint").options), ["auto", "on", "off"])
  assert.equal(IS.spec("idleIconTint").def, "auto")
  assert.equal(IS.spec("idleMotion").def, true)
  for (const k of ["idleIcon", "idleIconTint", "idleMotion"]) assert.equal(IS.spec(k).section, "look")
  const n = plain(IS.normalize({ idleNotch: "icon", idleIconTint: "wat", idleMotion: "false" }))
  assert.equal(n.idleNotch, "icon")
  assert.equal(n.idleIcon, "glasses")
  assert.equal(n.idleIconTint, "auto")
  assert.equal(n.idleMotion, false)
  assert.equal(plain(IS.withValue({}, "idleIcon", "swell")).idleIcon, "swell")
  assert.equal("idleIcon" in plain(IS.withValue({ idleIcon: "swell" }, "idleIcon", "glasses")), false)
  // manifest.json carries the same defaults.
  const manifest = readJson("manifest.json").barWidget
  assert.equal(manifest.defaults.idleIcon, "glasses")
  assert.equal(manifest.defaults.idleMotion, true)
})

test("strings exist in English and Spanish", () => {
  for (const lang of ["en", "es"]) {
    const t = (k) => IM.t(lang, k)
    for (const k of ["opt_icon", "opt_on", "set_idleIcon", "hint_idleIcon", "set_idleIconTint", "hint_idleIconTint",
      "set_idleMotion", "hint_idleMotion", "opt_auto", "opt_off"])
      assert.notEqual(t(k), k, `${lang} ${k}`)
  }
  assert.match(IM.t("en", "hint_idleIcon"), /\.config\/omarchy\/island\/icons/)
})

test("glyph table: every moving part has four keyframes inside the viewBox", () => {
  for (const b of Ic.BUILTINS) {
    const g = Ic.glyph(b)
    assert.ok(g.w > 0 && g.h > 0, b)
    if (g.dot) {
      assert.equal(g.dot.keys.length, 4, b)
      assert.deepEqual(plain(g.dot.keys[0]), plain(g.dot.keys[3]), `${b} ends at rest`)
      for (const [x, y] of g.dot.keys) {
        assert.ok(x - g.dot.r >= 0 && x + g.dot.r <= g.w, `${b} x`)
        assert.ok(y - g.dot.r >= 0 && y + g.dot.r <= g.h, `${b} y`)
      }
    }
    if (g.blink) assert.ok(g.blink.x + g.blink.w <= g.w && g.blink.y + g.blink.h <= g.h, b)
  }
  assert.deepEqual(plain(Ic.BUILTINS.filter((b) => Ic.hasDetail(Ic.glyph(b)))),
    ["glasses", "isle", "prompt", "capsule", "braces", "mark"])
  assert.equal(Ic.hasDetail(null), false)
  // The glasses' dot colour is the detail one; the others use the tint.
  assert.equal(Ic.glyph("glasses").dot.detail, true)
  assert.equal(!!Ic.glyph("capsule").dot.detail, false)
})

test("sample: keyframes at -, 0, 1, 2, 3 and between, clamped", () => {
  const dot = (n, p) => plain(Ic.sample(Ic.glyph(n).dot.keys, p))
  assert.deepEqual(dot("glasses", 0), [8, 6])
  assert.deepEqual(dot("glasses", 1), [10.5, 6])
  assert.deepEqual(dot("glasses", 2), [24, 6])
  assert.deepEqual(dot("glasses", 3), [8, 6])
  assert.deepEqual(dot("glasses", 1.5), [17.25, 6])
  assert.deepEqual(dot("glasses", -4), [8, 6])
  assert.deepEqual(dot("glasses", 9), [8, 6])
  assert.deepEqual(dot("capsule", 1), [24.5, 6])
  assert.deepEqual(dot("capsule", 2), [7.5, 6])
  assert.deepEqual(dot("braces", 1), [20.5, 6])
  assert.deepEqual(dot("braces", 2), [11.5, 6])
  assert.deepEqual(dot("mark", 0), [12, 12])
  assert.deepEqual(dot("mark", 1), [15, 12])
  assert.deepEqual(dot("mark", 2), [9, 12])
  assert.deepEqual(dot("mark", 2.5), [10.5, 12])
  assert.deepEqual(dot("mark", NaN), [12, 12])
  assert.deepEqual([0, 1, 1.5, 2, 3].map(Ic.sway), [0, 1, 0, -1, 0])
})

test("isle: mound control points follow the phase, wave shifts one unit", () => {
  const nums = (d) => d.match(/-?\d+(?:\.\d+)?/g).map(Number)
  const rest = "M 4 10.5 C 5.5 5.5 9 3.5 12 3.5 C 15 3.5 18.5 5.5 20 10.5 Z"
  assert.equal(Ic.isleMound(0, 1), rest)
  assert.equal(Ic.isleMound(3, 1), rest)
  assert.equal(Ic.isleMound(1, 1), "M 4 10.5 C 7 7 10.5 3.5 14 3.5 C 16 3.5 19.5 6 20 10.5 Z")
  assert.equal(Ic.isleMound(2, 1), "M 4 10.5 C 4.5 6 8 3.5 10 3.5 C 13.5 3.5 17 7 20 10.5 Z")
  // Half way is the mean of rest and right.
  assert.deepEqual(nums(Ic.isleMound(0.5, 1)).slice(2, 4), [6.25, 6.25])
  // Scaled by px per unit.
  assert.deepEqual(nums(Ic.isleMound(0, 2)).slice(0, 2), [8, 21])
  assert.equal(Ic.isleWave(0, 1), "M 3 13.5 q 2.25 -2 4.5 0 t 4.5 0 t 4.5 0 t 4.5 0")
  assert.equal(Ic.isleWave(1, 1).startsWith("M 4 13.5"), true)
  assert.equal(Ic.isleWave(2, 1).startsWith("M 2 13.5"), true)
  assert.equal(Ic.isleWave(1, 2).startsWith("M 8 27"), true)
})

test("the prompt's cursor sits after the SVG's underscore", () => {
  const b = Ic.glyph("prompt").blink
  assert.deepEqual(plain(b), { x: 20.5, y: 8.5, w: 6.5, h: 2, r: 1 })
})
