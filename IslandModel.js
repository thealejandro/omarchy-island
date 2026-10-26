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
