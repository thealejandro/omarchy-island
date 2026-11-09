// Microphone and camera in use: made-up PipeWire nodes only.
import { test } from "node:test"
import assert from "node:assert/strict"
import { load, plain } from "./load.mjs"

const PM = load("PrivacyModel.js")
const IS = load("IslandSettings.js")
const IM = load("IslandModel.js")

const node = (props, extra = {}) => ({ name: props["node.name"] || "", isStream: true, isSink: false, props, linkSources: [], ...extra })
const recorder = (app, extra = {}) => node({ "media.class": "Stream/Input/Audio", "application.name": app }, extra)

test("an app recording from the microphone counts", () => {
  assert.equal(PM.classify(recorder("Firefox")), "mic")
  assert.equal(PM.appName(recorder("Firefox")), "Firefox")
})

test("level meters, mixers and sink monitors don't", () => {
  assert.equal(PM.classify(node({ "media.class": "Stream/Input/Audio", "node.name": "quickshell-peak-monitor",
    "application.name": "Quickshell Peak Detect", "media.category": "Monitor", "stream.monitor": "true" })), null)
  assert.equal(PM.classify(recorder("pavucontrol")), null)
  assert.equal(PM.classify(recorder("PulseAudio Volume Control")), null)
  assert.equal(PM.classify(recorder("OBS", { linkSources: [{ isSink: true }] })), null)
  assert.equal(PM.classify(node({ "media.class": "Stream/Input/Audio", "media.name": "Peak detect", "application.name": "x" })), null)
})

test("playback and devices are not recording", () => {
  assert.equal(PM.classify(node({ "media.class": "Stream/Output/Audio", "application.name": "Spotify" })), null)
  assert.equal(PM.classify(node({ "media.class": "Audio/Source" }, { isStream: false })), null)
  assert.equal(PM.classify(null), null)
})

test("cameras: a video capture stream, or a source someone holds", () => {
  assert.equal(PM.classify(node({ "media.class": "Stream/Input/Video", "application.name": "Zoom" })), "camera")
  assert.equal(PM.classify(node({ "media.class": "Video/Source" }, { isStream: false })), null)
  assert.equal(PM.classify(node({ "media.class": "Video/Source" }, { isStream: false, inUse: true })), "camera")
})

test("names: application, then binary, then description; one entry per app", () => {
  assert.equal(PM.appName(node({ "application.process.binary": "obs" })), "Obs")
  assert.equal(PM.appName(node({ "node.description": "Grabadora" })), "Grabadora")
  assert.equal(PM.appName({ name: "raw-node", props: {} }), "Raw-node")
  const s = PM.summarize([recorder("Firefox"), recorder("firefox"), recorder("Discord"),
    node({ "media.class": "Stream/Input/Video", "application.name": "Discord" })], ["ffplay", "pipewire", ""])
  assert.deepEqual(plain(s), { mic: ["Discord", "Firefox"], camera: ["Discord", "Ffplay"] })
  assert.deepEqual(plain(PM.summarize(null, null)), { mic: [], camera: [] })
})

test("parseFuser reads pid<TAB>name lines", () => {
  assert.deepEqual(plain(PM.parseFuser("")), [])
  assert.deepEqual(plain(PM.parseFuser("1234\tffplay\n")), ["ffplay"])
  assert.deepEqual(plain(PM.parseFuser("1\tobs\n2\tchromium\nbasura\n\t\n")), ["obs", "chromium"])
})

test("label and started", () => {
  assert.equal(PM.label({ mic: [], camera: [] }), null)
  assert.deepEqual(plain(PM.label({ mic: ["Firefox"], camera: [] })), { key: "micInUse", arg: "Firefox" })
  assert.deepEqual(plain(PM.label({ mic: [], camera: ["OBS"] })), { key: "cameraInUse", arg: "OBS" })
  assert.deepEqual(plain(PM.label({ mic: ["Zoom"], camera: ["Zoom", "OBS"] })), { key: "micCameraInUse", arg: "Zoom, OBS" })
  const none = { mic: [], camera: [] }
  assert.equal(PM.started(none, { mic: ["Firefox"], camera: [] }), true)
  assert.equal(PM.started({ mic: ["Firefox"], camera: [] }, { mic: ["firefox"], camera: [] }), false)
  assert.equal(PM.started({ mic: ["Firefox"], camera: [] }, none), false)
  assert.equal(PM.started(null, { mic: [], camera: ["OBS"] }), true)
})

test("settings and strings", () => {
  for (const key of ["privacyDots", "privacyPeek"]) {
    const s = IS.spec(key)
    assert.equal(s.type, "bool")
    assert.equal(s.def, true)
    assert.equal(s.section, "system")
  }
  for (const k of ["micInUse", "cameraInUse", "micCameraInUse", "set_privacyDots", "set_privacyPeek", "hint_privacyDots"]) {
    for (const lang of ["en", "es"]) assert.notEqual(IM.t(lang, k), k, lang + ": " + k)
  }
})
