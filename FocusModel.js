.pragma library

// Deadline-based Pomodoro. The UI supplies time; no polling or side effects.
// Matches the native desktop cycle: breaks follow focus automatically,
// the next focus waits, and skipping preserves running/paused state.
function lengths(cfg) {
  return { focus: cfg.focusMinutes, break: cfg.breakMinutes,
    longBreak: cfg.longBreakMinutes, every: cfg.longBreakEvery }
}

function create(l) {
  return { phase: "focus", rounds: 0, running: false, deadline: 0,
    leftMs: l.focus * 60000, totalMs: l.focus * 60000 }
}

function remaining(s, now) {
  return Math.max(0, s.running ? s.deadline - now : s.leftMs)
}

function copy(s) {
  return { phase: s.phase, rounds: s.rounds, running: s.running,
    deadline: s.deadline, leftMs: s.leftMs, totalMs: s.totalMs }
}

function toggle(s, now) {
  var out = copy(s)
  out.leftMs = remaining(s, now)
  out.running = !s.running
  out.deadline = out.running ? now + out.leftMs : 0
  return out
}

function active(s) {
  return s.running || s.phase !== "focus" || s.leftMs !== s.totalMs
}

function advance(s, now, l) {
  var out = copy(s)
  if (s.phase === "focus") {
    out.rounds++
    out.phase = out.rounds >= l.every ? "longBreak" : "break"
  } else {
    if (s.phase === "longBreak") out.rounds = 0
    out.phase = "focus"
    out.running = false
  }
  out.totalMs = l[out.phase] * 60000
  out.leftMs = out.totalMs
  out.deadline = out.running ? now + out.totalMs : 0
  return out
}

function finish(s, now, l) {
  if (!s.running || now < s.deadline) return s
  // After suspend, provide a full break instead of replaying missed rounds.
  return advance(s, Math.max(s.deadline, now - 1000), l)
}

function resize(s, l) {
  if (s.running || s.leftMs !== s.totalMs) return s
  var out = copy(s)
  out.totalMs = l[out.phase] * 60000
  out.leftMs = out.totalMs
  return out
}
