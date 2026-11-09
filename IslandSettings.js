.pragma library

// Every setting the island reads from its entry in shell.json, in one place:
// the service normalizes the raw entry with it (anything invalid falls back
// to its default), the settings page draws its form from it, and
// manifest.json carries a copy for Omarchy's registry (toManifestSchema).
//
// type: "bool" | "enum" | "int" | "string" | "tristate" (unset/true/false)
//       | "weekday" (unset/1..7) | "cities" ([{name, zone}])

var SECTIONS = ["general", "look", "open", "bar", "world", "agenda", "notify", "system"]

var SCHEMA = [
  { key: "language", type: "enum", options: ["", "en", "es"], def: "", section: "general" },
  { key: "defaultPage", type: "enum", options: ["home", "calendar", "world"], def: "home", section: "general" },
  { key: "calendarFirst", type: "bool", def: false, section: "general" },
  { key: "showMedia", type: "bool", def: true, section: "general" },
  { key: "lyrics", type: "bool", def: false, section: "general" },
  { key: "lyricsPeek", type: "bool", def: false, section: "general" },
  { key: "showTimer", type: "bool", def: true, section: "general" },
  { key: "showDock", type: "bool", def: true, section: "general" },

  { key: "openWidth", type: "int", min: 420, max: 800, step: 20, def: 560, section: "look" },
  { key: "blur", type: "bool", def: true, section: "look" },
  { key: "animations", type: "bool", def: true, section: "look" },
  { key: "idleNotch", type: "enum", options: ["off", "notch", "hour", "day", "battery"], def: "notch", section: "look" },

  { key: "clock", type: "bool", def: true, section: "open" },
  { key: "clockClick", type: "enum", options: ["pin", "calendar", "omarchy"], def: "calendar", section: "open" },
  { key: "anchor", type: "enum", options: ["clock", "widget", "center"], def: "clock", section: "open" },
  { key: "clockWidth", type: "int", min: 60, max: 400, step: 10, def: 120, section: "open" },
  { key: "hoverOpen", type: "bool", def: true, section: "open" },
  { key: "keyboard", type: "bool", def: true, section: "open" },
  { key: "hoverDelay", type: "int", min: 0, max: 600, step: 20, def: 120, section: "open" },
  { key: "peekDuration", type: "int", min: 1000, max: 10000, step: 500, def: 3500, section: "open" },

  { key: "toggles", type: "enum", options: ["island", "both", "bar"], def: "island", section: "bar" },
  { key: "barClicks", type: "bool", def: true, section: "bar" },
  { key: "widgetIcon", type: "bool", def: false, section: "bar" },

  { key: "units", type: "enum", options: ["metric", "imperial"], def: "metric", section: "world" },
  { key: "forecast", type: "bool", def: true, section: "world" },
  { key: "hour24", type: "tristate", def: null, section: "world" },
  { key: "cities", type: "cities", def: [], section: "world" },

  { key: "weekStart", type: "weekday", def: null, section: "agenda" },
  { key: "reminderLead", type: "int", min: 0, max: 120, step: 5, def: 10, section: "agenda" },
  { key: "icsSources", type: "string", def: "", section: "agenda" },
  { key: "icsRefresh", type: "int", min: 5, max: 240, step: 5, def: 15, section: "agenda" },
  { key: "agendaCommand", type: "string", def: "", section: "agenda" },

  { key: "notifyBody", type: "bool", def: true, section: "notify" },
  { key: "notifyPeekDuration", type: "int", min: 2000, max: 15000, step: 500, def: 6000, section: "notify" },
  { key: "notifyHistory", type: "bool", def: true, section: "notify" },
  { key: "notifyHistoryLimit", type: "int", min: 20, max: 500, step: 20, def: 100, section: "notify" },
  { key: "notifyBadge", type: "bool", def: true, section: "notify" },
  { key: "notifyKeyboardIdle", type: "int", min: 3000, max: 30000, step: 1000, def: 10000, section: "notify" },

  { key: "notifications", type: "enum", options: ["off", "island"], def: "off", section: "system" },
  { key: "osd", type: "bool", def: false, section: "system" },
  { key: "battery", type: "bool", def: true, section: "system" },
  { key: "privacyDots", type: "bool", def: true, section: "system" },
  { key: "privacyPeek", type: "bool", def: true, section: "system" },
  { key: "agents", type: "bool", def: true, section: "system" },
  { key: "agentPeek", type: "bool", def: true, section: "system" },
  { key: "agentPeekDuration", type: "int", min: 2000, max: 15000, step: 500, def: 5000, section: "system" },
  { key: "agentDot", type: "bool", def: true, section: "system" },
  { key: "agentStale", type: "int", min: 10, max: 480, step: 10, def: 120, section: "system" }
]

var ZONE_ID = /^[A-Za-z_]+(\/[A-Za-z0-9_+\-]+)*$/

function spec(key) {
  for (var i = 0; i < SCHEMA.length; i++) if (SCHEMA[i].key === key) return SCHEMA[i]
  return null
}

function inSection(section) {
  return SCHEMA.filter(function(s) { return s.section === section })
}

function asBool(v) {
  if (v === true || v === "true") return true
  if (v === false || v === "false") return false
  return undefined
}

// [{name, zone}] from an array of such objects or from "Name|Zone, ..."
// text (the format the settings field and elsewhen's legacy `zones` use).
function parseCities(raw) {
  var list = []
  if (Array.isArray(raw)) {
    list = raw
  } else if (typeof raw === "string") {
    list = raw.split(/[,\n]/).map(function(part) {
      var bits = part.split("|")
      return bits.length > 1 ? { name: bits[0], zone: bits[1] } : { name: "", zone: bits[0] }
    })
  }
  var out = []
  for (var i = 0; i < list.length; i++) {
    var c = list[i]
    if (!c || typeof c !== "object") continue
    var zone = String(c.zone || c.id || "").trim()
    if (zone === "" || !ZONE_ID.test(zone)) continue
    var name = String(c.name || c.label || "").trim()
    if (name === "") name = zone.split("/").pop().replace(/_/g, " ")
    out.push({ name: name, zone: zone })
  }
  return out
}

function citiesText(cities) {
  return (cities || []).map(function(c) { return c.name + "|" + c.zone }).join(", ")
}

function normalizeOne(s, v) {
  switch (s.type) {
    case "bool": {
      var b = asBool(v)
      return b === undefined ? s.def : b
    }
    case "enum":
      return s.options.indexOf(v) !== -1 ? v : s.def
    case "int": {
      var n = parseInt(v, 10)
      if (isNaN(n)) return s.def
      return Math.max(s.min, Math.min(s.max, n))
    }
    case "string":
      return v === undefined || v === null ? s.def : String(v)
    case "tristate": {
      var t = asBool(v)
      return t === undefined ? null : t
    }
    case "weekday": {
      var d = parseInt(v, 10)
      return isNaN(d) || d < 1 || d > 7 ? null : d
    }
    case "cities":
      return parseCities(v)
  }
  return s.def
}

// The raw shell.json entry → a complete settings object. Unknown keys are
// dropped here (they stay untouched in shell.json).
function normalize(raw) {
  var src = raw && typeof raw === "object" ? raw : {}
  var out = {}
  for (var i = 0; i < SCHEMA.length; i++) out[SCHEMA[i].key] = normalizeOne(SCHEMA[i], src[SCHEMA[i].key])
  return out
}

function isDefault(key, value) {
  var s = spec(key)
  if (!s) return false
  return JSON.stringify(normalizeOne(s, value)) === JSON.stringify(s.def)
}

// The entry to write after changing `key`: the raw entry as it is (unknown
// keys kept), the new value normalized, and defaults left out so shell.json
// only holds what was actually changed.
function withValue(raw, key, value, id) {
  var entry = { id: id }
  var src = raw && typeof raw === "object" ? raw : {}
  for (var k in src) if (k !== "id") entry[k] = src[k]
  var s = spec(key)
  var v = s ? normalizeOne(s, value) : value
  if (s && isDefault(key, v)) delete entry[key]
  else entry[key] = v
  return entry
}

// Only the island's own keys are dropped; anything else in the entry stays.
function withoutOwnKeys(raw, id) {
  var entry = { id: id }
  var src = raw && typeof raw === "object" ? raw : {}
  for (var k in src) if (k !== "id" && !spec(k)) entry[k] = src[k]
  return entry
}

// barWidget.defaults / barWidget.schema for manifest.json, in the shape
// Omarchy's first-party widgets use. label(key) gives the English label.
function toManifestSchema(label) {
  var defaults = {}, schema = []
  for (var i = 0; i < SCHEMA.length; i++) {
    var s = SCHEMA[i]
    // Unset (null, [], or "" for the language) means "decide on the fly",
    // which has no value to show.
    var hasDefault = s.def !== null && !(Array.isArray(s.def) && s.def.length === 0)
      && !(s.type === "enum" && s.def === "")
    if (hasDefault) defaults[s.key] = s.def
    var type = s.type === "bool" || s.type === "tristate" ? "boolean"
      : (s.type === "int" || s.type === "weekday" ? "integer"
        : (s.type === "enum" ? "enum" : "string"))
    // cities is a list in shell.json, but "Name|Zone, …" text is read too,
    // and that's what a generic editor can offer.
    var item = { key: s.key, type: type, label: label ? label(s.key) : s.key }
    if (s.options) item.options = s.options.filter(function(o) { return o !== "" })
    if (s.type === "int") { item.min = s.min; item.max = s.max; item.step = s.step }
    if (s.type === "weekday") { item.min = 1; item.max = 7 }
    if (hasDefault) item.defaultValue = s.def
    schema.push(item)
  }
  return { defaults: defaults, schema: schema }
}
