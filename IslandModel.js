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
    scrollToChange: "Scroll to change"
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
    scrollToChange: "Rueda para cambiar"
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
