.pragma library

// Who is using the microphone or the camera, worked out from PipeWire
// nodes (and, for cameras opened straight through /dev/video*, from fuser).
// Pure, so it can be tested outside Quickshell.
//
// A node comes in as { name, isStream, isSink, props: { "media.class", …},
// linkSources: [{ isSink }] }: what Service reads from a PwNode and the
// link groups it takes its input from.

// Level meters and patchbays capture without anyone recording.
var IGNORED_APPS = ["quickshell", "quickshell peak detect", "pulseaudio volume control", "pavucontrol",
  "cava", "helvum", "qpwgraph", "pw-top", "pw-dump", "easyeffects", "wireplumber"]
var METER_NODES = ["quickshell", "quickshell-peak-monitor"]
var METER_CATEGORIES = ["monitor", "manager"]

function prop(node, key) {
  var p = node && node.props ? node.props[key] : undefined
  return p === undefined || p === null ? "" : String(p)
}
function low(s) { return String(s || "").trim().toLowerCase() }

// A stream that only listens to levels or to what's playing (a sink's
// monitor) isn't recording you.
function isMonitorStream(node) {
  if (low(prop(node, "stream.monitor")) === "true") return true
  if (METER_CATEGORIES.indexOf(low(prop(node, "media.category"))) !== -1) return true
  if (low(prop(node, "media.name")) === "peak detect") return true
  if (METER_NODES.indexOf(low(node.name)) !== -1) return true
  var src = node.linkSources || []
  for (var i = 0; i < src.length; i++) if (src[i] && src[i].isSink) return true
  return false
}

function isIgnoredApp(node) {
  var names = [prop(node, "application.name"), prop(node, "application.process.binary")]
  for (var i = 0; i < names.length; i++) if (IGNORED_APPS.indexOf(low(names[i])) !== -1) return true
  return false
}

// "mic" | "camera" | null.
function classify(node) {
  if (!node) return null
  var cls = prop(node, "media.class")
  var kind = null
  if (cls === "Stream/Input/Audio" || (cls === "" && node.isStream && !node.isSink)) kind = "mic"
  else if (cls === "Stream/Input/Video") kind = "camera"
  else if (cls === "Video/Source" && (node.linkSources || []).length === 0 && node.inUse) kind = "camera"
  if (!kind) return null
  if (isIgnoredApp(node)) return null
  if (kind === "mic" && isMonitorStream(node)) return null
  return kind
}

// The name people know the app by.
function appName(node) {
  var keys = ["application.name", "application.process.binary", "node.description", "node.nick"]
  for (var i = 0; i < keys.length; i++) {
    var v = prop(node, keys[i]).trim()
    if (v !== "") return pretty(v)
  }
  return pretty(String(node && node.name || ""))
}

// "firefox" → "Firefox"; names with capitals are left alone.
function pretty(name) {
  var s = String(name || "").trim()
  if (s === "") return ""
  if (s === s.toLowerCase()) s = s.charAt(0).toUpperCase() + s.slice(1)
  return s
}

function addUnique(list, name) {
  if (name === "") return
  for (var i = 0; i < list.length; i++) if (low(list[i]) === low(name)) return
  list.push(name)
}

// Nodes (and camera process names from fuser) → { mic: [...], camera: [...] }.
function summarize(nodes, cameraProcs) {
  var out = { mic: [], camera: [] }
  var list = Array.isArray(nodes) ? nodes : []
  for (var i = 0; i < list.length; i++) {
    var kind = classify(list[i])
    if (kind) addUnique(out[kind], appName(list[i]))
  }
  var procs = Array.isArray(cameraProcs) ? cameraProcs : []
  for (var k = 0; k < procs.length; k++) {
    var n = String(procs[k] || "").trim()
    if (n === "" || IGNORED_APPS.indexOf(low(n)) !== -1) continue
    // The PipeWire daemon holding the device is PipeWire, not an app.
    if (["pipewire", "wireplumber", "pipewire-media-session"].indexOf(low(n)) !== -1) continue
    addUnique(out.camera, pretty(n))
  }
  out.mic.sort(); out.camera.sort()
  return out
}

// The script's output: "pid<TAB>comm" lines → process names.
function parseFuser(text) {
  var out = []
  var lines = String(text || "").split("\n")
  for (var i = 0; i < lines.length; i++) {
    var f = lines[i].split("\t")
    if (f.length < 2 || !/^\d+$/.test(f[0].trim())) continue
    var name = f.slice(1).join(" ").trim()
    if (name !== "") out.push(name)
  }
  return out
}

// What the glance says: { key, arg } or null.
function label(summary) {
  if (!summary) return null
  var mic = summary.mic || [], cam = summary.camera || []
  if (mic.length && cam.length) {
    var both = mic.slice()
    for (var i = 0; i < cam.length; i++) addUnique(both, cam[i])
    return { key: "micCameraInUse", arg: both.join(", ") }
  }
  if (mic.length) return { key: "micInUse", arg: mic.join(", ") }
  if (cam.length) return { key: "cameraInUse", arg: cam.join(", ") }
  return null
}

// Has an app started using either since last time? (Not when one stops.)
function started(prev, next) {
  var kinds = ["mic", "camera"]
  for (var k = 0; k < kinds.length; k++) {
    var before = (prev && prev[kinds[k]]) || []
    var now = (next && next[kinds[k]]) || []
    for (var i = 0; i < now.length; i++) {
      var seen = false
      for (var j = 0; j < before.length; j++) if (low(before[j]) === low(now[i])) seen = true
      if (!seen) return true
    }
  }
  return false
}
