.pragma library

// Presentation policy for the Omarchy adapter: pure and tested independently.
function fitWidth(setting, screen, scale, margin) {
  scale = Math.max(0.1, Number(scale) || 1)
  screen = Math.max(0, Number(screen) || 0)
  margin = Math.max(0, Number(margin) || 0)
  var available = Math.max(0, screen - 2 * margin)
  var most = Math.floor(screen * 0.6 / (10 * scale)) * 10 * scale
  // Preserve the plugin's existing 420px settings on upgrade.
  return Math.max(0, Math.min(setting * scale, Math.max(420 * scale, most), available))
}

// The agenda moves below the music with a column, keeping both readable.
function lyricsWidth(content, gap, playerMin, lyricsMin) {
  var room = Math.max(0, content - gap)
  if (room < playerMin + lyricsMin) return 0
  return Math.min(room - playerMin, Math.max(lyricsMin, Math.round(room * 0.42)))
}

function songLine(title, artist, mode) {
  title = String(title || "")
  artist = String(artist || "")
  if (mode === "title") return title
  if (mode === "artist") return artist || title
  var a = mode === "artist-title" ? artist : title
  var b = mode === "artist-title" ? title : artist
  return a && b ? a + "  ·  " + b : a || b
}

// Never build hundreds of Text items. A virtual pause covers the prelude.
function lyricRows(lines, index) {
  if (!Array.isArray(lines) || lines.length === 0) return []
  var current = Math.max(-1, Math.min(lines.length - 1, index))
  var out = []
  for (var i = Math.max(-1, current - 2); i <= Math.min(lines.length - 1, current + 4); i++) {
    if (i === -1 && current !== -1) continue
    out.push({ index: i, text: i < 0 ? "" : lines[i].text, distance: i - current })
  }
  return out
}

function lyricAlpha(distance) {
  return distance < 0 ? Math.max(0.16, 1 + distance * 0.62)
    : Math.max(0.2, 1 - distance * 0.42)
}
