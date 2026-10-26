.pragma library

// Prefer whatever is playing right now; otherwise the first player that has
// a track loaded, so a paused song stays reachable.
function pickPlayer(players) {
  if (!players || players.length === 0) return null
  var withTrack = null
  for (var i = 0; i < players.length; i++) {
    var p = players[i]
    if (!p) continue
    if (p.isPlaying) return p
    if (!withTrack && (p.trackTitle || p.trackArtist)) withTrack = p
  }
  return withTrack
}

function clock(seconds) {
  var s = Math.max(0, Math.floor(seconds || 0))
  var m = Math.floor(s / 60)
  var r = s % 60
  return m + ":" + (r < 10 ? "0" : "") + r
}

// ------------------------------------------------------------- language

var STRINGS = {
  en: {
    nothingPlaying: "Nothing playing",
    nothingPlayingHint: "Play something in Spotify, a browser or mpv",
    focus: "Focus",
    break: "Break",
    focusDone: "Focus done",
    takeBreak: "Take a %1 minute break.",
    breakOver: "Break over",
    readyAgain: "Ready for another round?",
    today: "Today",
    minutes: "%1 min",
    scrollToChange: "Scroll to change",
    nightLight: "Night light",
    doNotDisturb: "Silence",
    stayAwake: "Stay awake",
    recording: "Record",
    stopRecording: "Stop",
    reminders: "Reminder",
    screenshot: "Screenshot",
    colorPicker: "Color picker",
    clipboard: "Clipboard",
    emoji: "Emoji",
    weather: "Weather"
  },
  es: {
    nothingPlaying: "Nada sonando",
    nothingPlayingHint: "Pon algo en Spotify, el navegador o mpv",
    focus: "Enfoque",
    break: "Descanso",
    focusDone: "Enfoque terminado",
    takeBreak: "Tómate un descanso de %1 minutos.",
    breakOver: "Descanso terminado",
    readyAgain: "¿Otra ronda?",
    today: "Hoy",
    minutes: "%1 min",
    scrollToChange: "Rueda para cambiar",
    nightLight: "Luz nocturna",
    doNotDisturb: "Silencio",
    stayAwake: "Despierto",
    recording: "Grabar",
    stopRecording: "Detener",
    reminders: "Recordatorio",
    screenshot: "Captura",
    colorPicker: "Cuentagotas",
    clipboard: "Portapapeles",
    emoji: "Emoji",
    weather: "Clima"
  }
}

// "es_ES.UTF-8", "es-MX" or "es" all resolve to "es"; anything we don't
// translate falls back to English.
function language(code) {
  var base = String(code || "").toLowerCase().split(/[_.@-]/)[0]
  return STRINGS[base] ? base : "en"
}

function t(lang, key, arg) {
  var table = STRINGS[lang] || STRINGS.en
  var s = table[key] !== undefined ? table[key] : (STRINGS.en[key] || key)
  return arg !== undefined ? s.replace("%1", arg) : s
}

// ------------------------------------------------------------- calendar

// 6 weeks x 7 days starting on the locale's first weekday, so every month
// has the same height and the island doesn't jump when paging through.
// firstDow uses Qt's numbering: 1 = Monday ... 7 = Sunday.
function monthGrid(year, month, firstDow, today) {
  var first = new Date(year, month, 1)
  var jsFirst = first.getDay()                 // 0 = Sunday
  var qtFirst = jsFirst === 0 ? 7 : jsFirst
  var lead = (qtFirst - firstDow + 7) % 7
  var cells = []
  for (var i = 0; i < 42; i++) {
    var d = new Date(year, month, 1 - lead + i)
    cells.push({
      day: d.getDate(),
      inMonth: d.getMonth() === month,
      isToday: d.getFullYear() === today.getFullYear()
        && d.getMonth() === today.getMonth()
        && d.getDate() === today.getDate(),
      weekend: d.getDay() === 0 || d.getDay() === 6
    })
  }
  return cells
}

// Qt day numbers (1..7) for the header row, starting at firstDow.
function weekdayOrder(firstDow) {
  var out = []
  for (var i = 0; i < 7; i++) out.push(((firstDow - 1 + i) % 7) + 1)
  return out
}

// wttr.in's j1 report → { temp, code, desc, place }. lang picks wttr's
// translated description ("lang_es") when it has one. Anything malformed
// returns null so the caller keeps the last good report.
function parseWeather(raw, lang, imperial) {
  try {
    var data = JSON.parse(String(raw || ""))
    var now = data.current_condition[0]
    var translated = now["lang_" + lang]
    var desc = translated && translated[0] ? translated[0].value
      : (now.weatherDesc && now.weatherDesc[0] ? now.weatherDesc[0].value : "")
    var area = data.nearest_area && data.nearest_area[0] && data.nearest_area[0].areaName
      ? data.nearest_area[0].areaName[0].value : ""
    var temp = imperial ? now.temp_F : now.temp_C
    if (temp === undefined || temp === "") return null
    return { temp: temp + "°", code: parseInt(now.weatherCode, 10), desc: desc.trim(), place: area }
  } catch (e) {
    return null
  }
}

// Nerd Font icon for a wttr.in weather code, the same set Omarchy's weather
// widget uses.
function weatherIcon(code, night) {
  switch (code) {
    case 113: return night ? "\ue32b" : "\ue30d"
    case 116: return night ? "\ue32e" : "\ue302"
    case 119: case 122: return "\ue33d"
    case 143: case 248: case 260: return night ? "\ue346" : "\ue313"
    case 176: case 263: case 353: return night ? "\ue333" : "\ue308"
    case 179: case 227: case 230: case 323: case 326: case 368: return night ? "\ue327" : "\ue30a"
    case 182: case 185: case 281: case 284: case 311: case 314:
    case 317: case 320: case 350: case 362: case 365: case 374: case 377: return "\ue3ad"
    case 200: case 386: case 389: case 392: case 395: return "\ue31d"
    case 266: case 293: case 296: case 299: case 302: case 305: case 308: case 356: case 359: return "\ue318"
    case 329: case 332: case 335: case 338: case 371: return "\ue31a"
    default: return "\ue33d"
  }
}
