.pragma library

// What the closed island draws under the clock: a notch (a sliver of the bar)
// and, optionally, a thread inside it: the hour, the day (15-minute steps) or
// the battery level.

var LOOKS = ["off", "notch", "hour", "day", "battery"]

// Something is already shown (or should be) under the clock with the island
// closed. Music is not counted: it is not shown there while the island is closed.
function busy(s) {
  if (!s) return false
  return !!(s.unread || s.mic || s.camera || s.agent || s.timer)
}

// "none" | "notch" | "hour" | "day" | "battery": what is drawn under the clock.
// The threads yield to anything busy; "battery" is a plain notch with no battery.
function look(setting, mode, barHidden, isBusy, hasBattery) {
  if (LOOKS.indexOf(setting) === -1 || setting === "off") return "none"
  if (mode !== "closed" || barHidden) return "none"
  if (isBusy || setting === "notch") return "notch"
  if (setting === "battery") return hasBattery ? "battery" : "notch"
  return setting
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

// 0..1 along the local day, in 15-minute steps; the middle of the step, so
// 00:00 is not exactly 0 nor 23:59 exactly 1.
function fractionOfDay(minutesOfDay) {
  var m = Number(minutesOfDay)
  if (!isFinite(m) || m < 0) return 0
  return (Math.min(95, Math.floor(Math.floor(m) / 15)) + 0.5) / 96
}

// 0..1 from a battery percentage; -1 (no battery) and junk give 0.
function fractionOfBattery(level) {
  var l = Number(level)
  if (!isFinite(l)) return 0
  return Math.max(0, Math.min(100, l)) / 100
}

// Low battery: 20% or less and not charging.
function batteryLow(level, charging) {
  var l = Number(level)
  return isFinite(l) && l >= 0 && l <= 20 && !charging
}

// Ms until the next quarter hour from a local clock reading (minutes since
// any hour or day start, seconds, ms). Always 1..900000, so it can never spin.
// Exactly on a quarter it waits for the next one.
function msUntilNextQuarter(minutes, seconds, ms) {
  var m = Math.max(0, Math.floor(Number(minutes) || 0))
  var r = (m % 15) * 60000 + Math.max(0, Number(seconds) || 0) * 1000 + Math.max(0, Number(ms) || 0)
  r = r % 900000
  return 900000 - r
}
