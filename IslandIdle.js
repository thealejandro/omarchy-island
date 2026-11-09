.pragma library

// What the closed island draws under the clock: a notch (a sliver of the bar)
// and, optionally, the hour thread that fills it as the hour goes by.

var LOOKS = ["off", "notch", "hour"]

// Something is already shown (or should be) under the clock with the island
// closed. Music is not counted: it is not shown there while the island is closed.
function busy(s) {
  if (!s) return false
  return !!(s.unread || s.mic || s.camera || s.agent || s.timer)
}

// "none" | "notch" | "hour": what is drawn under the clock.
function look(setting, mode, barHidden, isBusy) {
  if (LOOKS.indexOf(setting) === -1 || setting === "off") return "none"
  if (mode !== "closed" || barHidden) return "none"
  return setting === "hour" && !isBusy ? "hour" : "notch"
}

// 0..1 along the local hour; (minute + 0.5) / 60 so minute 0 is not exactly 0.
function fractionOfMinute(minute) {
  var m = Number(minute)
  if (!isFinite(m) || m < 0) return 0
  return (Math.min(59, Math.floor(m)) + 0.5) / 60
}

// Thread length in px inside `length`, never below minPx nor above length.
function threadLength(fraction, length, minPx) {
  var f = Math.max(0, Math.min(1, Number(fraction) || 0))
  return Math.max(0, Math.min(length, Math.max(minPx, Math.round(f * length))))
}

// Ms until the next minute, in epoch ms (immune to DST changes, like Omarchy's
// WallClock). Always 1..60000, so it can never spin.
function msUntilNextMinute(nowMs) {
  var r = nowMs % 60000
  return r === 0 ? 60000 : 60000 - r
}
