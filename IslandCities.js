.pragma library
.import "IslandSettings.js" as IslandSettings

// Pure helpers for the editable city list in Settings › World: moving,
// removing and adding entries, and filtering the IANA zone list the add
// row's searcher shows. Tested from node (test/cities.test.mjs); the zone
// list itself is fetched by the editor widget (IslandCitiesEditor.qml).

var ZONE_ID = /^[A-Za-z_]+(\/[A-Za-z0-9_+\-]+)*$/

// Short list for when `timedatectl list-timezones` cannot run.
var FALLBACK_ZONES = [
  "America/Argentina/Buenos_Aires",
  "America/Bogota",
  "America/Chicago",
  "America/Lima",
  "America/Los_Angeles",
  "America/Mexico_City",
  "America/New_York",
  "America/Sao_Paulo",
  "America/Toronto",
  "Asia/Dubai",
  "Asia/Hong_Kong",
  "Asia/Kolkata",
  "Asia/Shanghai",
  "Asia/Singapore",
  "Asia/Tokyo",
  "Australia/Sydney",
  "Europe/Amsterdam",
  "Europe/Berlin",
  "Europe/Lisbon",
  "Europe/London",
  "Europe/Madrid",
  "Europe/Moscow",
  "Europe/Paris",
  "Europe/Rome",
  "Pacific/Auckland"
]

function isValidZone(zone) {
  return ZONE_ID.test(String(zone || "").trim())
}

// `timedatectl list-timezones` output (one zone per line) → valid zones,
// deduplicated and in order; anything that is not "Area/City" is dropped.
function parseZoneLines(text) {
  var seen = {}
  var out = []
  var lines = String(text || "").split("\n")
  for (var i = 0; i < lines.length; i++) {
    var z = lines[i].trim()
    if (!isValidZone(z) || seen[z]) continue
    seen[z] = true
    out.push(z)
  }
  return out
}

// Case-insensitive substring search over the zone list; every "/" boundary
// counts as a space too, so "europe madrid" finds Europe/Madrid. An empty
// query shows the first `limit` zones, so the searcher offers something
// before there is anything typed.
function filterZones(zones, query, limit) {
  var needle = String(query || "").trim().toLowerCase()
  var hay = needle === "" ? null : needle.replace(/\//g, " ")
  var max = limit === undefined ? 8 : limit
  var out = []
  var list = zones || []
  for (var i = 0; i < list.length && out.length < max; i++) {
    var z = String(list[i])
    var low = z.toLowerCase()
    if (needle === "" || low.indexOf(needle) !== -1 || low.replace(/\//g, " ").indexOf(hay) !== -1)
      out.push(z)
  }
  return out
}

// [{name, zone}] → copy with the entry at `index` moved `delta` places,
// stopping at both ends. An index outside the list leaves it untouched.
function move(cities, index, delta) {
  var list = (cities || []).slice()
  var i = Math.floor(index)
  if (!(i >= 0) || i >= list.length) return list
  var j = Math.max(0, Math.min(list.length - 1, i + delta))
  if (j === i) return list
  var item = list.splice(i, 1)[0]
  list.splice(j, 0, item)
  return list
}

// [{name, zone}] → copy without the entry at `index`.
function remove(cities, index) {
  var list = (cities || []).slice()
  var i = Math.floor(index)
  if (!(i >= 0) || i >= list.length) return list
  list.splice(i, 1)
  return list
}

// Appends a city. The zone must be valid; the name may be "", in which case
// the settings parser derives one from the zone (as it does for text input).
// An invalid zone returns null so the caller stays put.
function add(cities, name, zone) {
  var parsed = IslandSettings.parseCities([{ name: name, zone: zone }])
  if (parsed.length === 0) return null
  var list = (cities || []).slice()
  list.push(parsed[0])
  return list
}
