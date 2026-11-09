.pragma library
.import "NotificationSecurity.js" as Sec
.import "NotificationModel.js" as NM

// What AI coding agents (Claude Code, Codex, Gemini CLI, Aider…) are doing,
// as told by their hooks through IPC: one session per agent and session id.
// Pure, so it can be tested outside Quickshell: `now` is always passed in.
//
// A session is { key, agent, id, state, label, message, pid, address, since,
// updated, peeked }. The working directory is never kept: only the name of
// its last folder (label). Everything that comes in is untrusted text from
// another process, so it is cleaned before it is stored.

var STATES = ["working", "waiting", "done", "error", "idle"]
// null means "the session is over": it is dropped.
var ALIASES = {
  start: "working", busy: "working", running: "working", prompt: "working",
  permission: "waiting", ask: "waiting", input: "waiting",
  stop: "done", finished: "done", complete: "done",
  fail: "error", failed: "error",
  end: null, exit: null, clear: null
}
var MAX_SESSIONS = 12
var MAX_NAME = 32
var MAX_ID = 64
var MAX_LABEL = 48
var MAX_MESSAGE = 160
var MAX_PAYLOAD = 4096
var MAX_PID = 4194304
// How long a finished, failed or idle session stays in view.
var TTL = { done: 5 * 60000, error: 10 * 60000, idle: 60000 }
var NAMES = { claude: "Claude Code", codex: "Codex", gemini: "Gemini CLI", aider: "Aider",
              opencode: "OpenCode", cursor: "Cursor" }
// Nerd Font glyphs (as surrogate pairs): robot, check, close, hourglass, sleep.
var GLYPHS = { waiting: "󰚩", done: "󰄬", error: "󰅖",
               working: "󰔟", idle: "󰒲" }

function str(v) { return v === undefined || v === null ? "" : String(v) }

// A known state, null for "the session ended" (or its aliases), undefined
// for anything else.
function normalizeState(raw) {
  var s = str(raw).trim().toLowerCase()
  if (STATES.indexOf(s) !== -1) return s
  if (Object.prototype.hasOwnProperty.call(ALIASES, s)) return ALIASES[s]
  return undefined
}

function cleanName(raw) { return str(raw).toLowerCase().replace(/[^a-z0-9._-]/g, "").slice(0, MAX_NAME) }
function cleanId(raw) { return str(raw).replace(/[^A-Za-z0-9._:-]/g, "").slice(0, MAX_ID) }

// One line of plain text: escape sequences, control and direction
// characters gone, spaces collapsed, cut to `max`.
function cleanText(raw, max) {
  var s = str(raw)
    .replace(/\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)?/g, "")
    .replace(/\x1b\[[0-?]*[ -\/]*[@-~]/g, "")
    .replace(/\x1b[@-_]/g, "")
    .replace(/[\x00-\x1f\x7f-\x9f]/g, " ")
    .replace(/[​-‏‪-‮⁦-⁩﻿]/g, "")
    .replace(/\s+/g, " ")
    .trim()
  if (s.length > max) {
    s = s.slice(0, max)
    // Not in the middle of a surrogate pair.
    if (/[\ud800-\udbff]$/.test(s)) s = s.slice(0, -1)
    s = s.trim()
  }
  return s
}

function cleanPid(raw) {
  var s = str(raw).trim()
  if (!/^\d{1,8}$/.test(s)) return 0
  var n = parseInt(s, 10)
  return n >= 1 && n <= MAX_PID ? n : 0
}

function cleanAddress(raw) {
  var s = str(raw).trim()
  return /^0x[0-9a-f]{1,16}$/i.test(s) ? s : ""
}

// The last folder of a working directory, and nothing else of the path.
function projectLabel(cwd) {
  var parts = str(cwd).split(/[\/\\]+/)
  for (var i = parts.length - 1; i >= 0; i--) {
    var p = cleanText(parts[i], MAX_LABEL)
    if (p === "") continue
    return p === "." || p === ".." || p === "~" ? "" : p
  }
  return ""
}

function displayName(agent) {
  var a = cleanName(agent)
  if (a === "") return ""
  if (Object.prototype.hasOwnProperty.call(NAMES, a)) return NAMES[a]
  return a.charAt(0).toUpperCase() + a.slice(1)
}

function sessionKey(agent, id) { return agent + ":" + (id || "default") }

function glyph(state) { return GLYPHS[state] || GLYPHS.idle }

function copy(s) {
  return { key: s.key, agent: s.agent, id: s.id, state: s.state, label: s.label, message: s.message,
           pid: s.pid, address: s.address, since: s.since, updated: s.updated, peeked: s.peeked }
}

function indexOfKey(list, key) {
  for (var i = 0; i < list.length; i++) if (list[i].key === key) return i
  return -1
}

// Raw fields from IPC → clean ones, or null when there is no agent name.
// state stays null for "ended" and undefined for unknown ones.
function prepare(fields) {
  var f = fields && typeof fields === "object" ? fields : {}
  var agent = cleanName(f.agent)
  if (agent === "") return null
  var id = cleanId(f.id)
  return { agent: agent, id: id, key: sessionKey(agent, id), state: normalizeState(f.state),
           label: projectLabel(f.cwd), message: cleanText(f.message, MAX_MESSAGE),
           pid: cleanPid(f.pid), address: cleanAddress(f.address) }
}

function trim(list, keep) {
  var out = list.slice()
  while (out.length > MAX_SESSIONS) {
    var victim = -1
    // Oldest finished, failed or idle first, then working, then waiting.
    var tiers = [["done", "error", "idle"], ["working"], ["waiting"]]
    for (var t = 0; t < tiers.length && victim < 0; t++) {
      for (var i = 0; i < out.length; i++) {
        if (out[i].key === keep || tiers[t].indexOf(out[i].state) === -1) continue
        if (victim < 0 || out[i].updated <= out[victim].updated) victim = i
      }
    }
    if (victim < 0) break
    out.splice(victim, 1)
  }
  return out
}

// Applies one event to the list. Returns { list, session, changed, prev,
// removed, invalid }: a new list (the old one is left alone), the session as
// it is now (null when it was dropped), what it was before (or null), and
// whether anything visible changed.
function update(list, fields, now) {
  var cur = Array.isArray(list) ? list : []
  var f = prepare(fields)
  if (!f || f.state === undefined) return { list: cur, session: null, changed: false, prev: null, removed: false, invalid: true }
  var at = indexOfKey(cur, f.key)
  var prev = at >= 0 ? cur[at] : null
  if (f.state === null) {
    return { list: remove(cur, f.key), session: null, changed: at >= 0, prev: prev, removed: at >= 0, invalid: false }
  }
  var s = {
    key: f.key, agent: f.agent, id: f.id, state: f.state,
    label: f.label || (prev ? prev.label : ""),
    message: f.message,
    pid: f.pid || (prev ? prev.pid : 0),
    address: f.address || (prev ? prev.address : ""),
    since: prev && prev.state === f.state ? prev.since : now,
    updated: now,
    peeked: prev && prev.state === f.state ? prev.peeked : ""
  }
  var changed = !prev || prev.state !== s.state || prev.label !== s.label || prev.message !== s.message
    || prev.pid !== s.pid || prev.address !== s.address
  var next = [s]
  for (var i = 0; i < cur.length; i++) if (i !== at) next.push(cur[i])
  return { list: trim(next, s.key), session: copy(s), changed: changed, prev: prev ? copy(prev) : null,
           removed: false, invalid: false }
}

function remove(list, key) {
  return (list || []).filter(function(s) { return s.key !== key })
}

// All the sessions of one agent; "" for every agent.
function removeAgent(list, name) {
  var n = cleanName(name)
  return (list || []).filter(function(s) { return n !== "" && s.agent !== n })
}

// The window an agent runs in, when it turns up after the first event.
function setWindow(list, key, pid, address) {
  var p = cleanPid(pid), a = cleanAddress(address)
  var found = false
  var out = (list || []).map(function(s) {
    if (s.key !== key) return s
    found = true
    var c = copy(s)
    if (p) c.pid = p
    if (a) c.address = a
    return c
  })
  return { list: out, found: found && (p > 0 || a !== "") }
}

function markPeeked(list, key, state) {
  return (list || []).map(function(s) {
    if (s.key !== key) return s
    var c = copy(s)
    c.peeked = state
    return c
  })
}

// When a session goes away or, if it was working or waiting, is taken as
// stuck: ms from `updated`.
function lifetime(s, staleMs) {
  return TTL[s.state] !== undefined ? TTL[s.state] : staleMs
}

// Sessions past their time go: finished, failed and idle ones disappear,
// working and waiting ones nobody heard from for staleMs turn idle.
function prune(list, now, staleMs) {
  var out = []
  for (var i = 0; i < (list || []).length; i++) {
    var s = list[i]
    if (now - s.updated < lifetime(s, staleMs)) { out.push(s); continue }
    if (TTL[s.state] !== undefined) continue
    var c = copy(s)
    c.state = "idle"
    c.since = now
    c.updated = now
    c.message = ""
    out.push(c)
  }
  return out
}

// ms until the next session expires; 0 when there is nothing to wait for.
function nextPruneDelay(list, now, staleMs) {
  var best = 0
  for (var i = 0; i < (list || []).length; i++) {
    var d = Math.max(1, list[i].updated + lifetime(list[i], staleMs) - now)
    if (best === 0 || d < best) best = d
  }
  return best
}

// Does this change deserve a glance? Needing you does (once per time it
// starts), finishing and failing do; working and idle never; and the same
// state of the same session is never announced twice.
function shouldPeek(prev, next) {
  if (!next) return false
  if (next.peeked === next.state) return false
  if (prev && prev.state === next.state) return false
  return next.state === "waiting" || next.state === "done" || next.state === "error"
}

function sessionLabel(s) {
  return displayName(s.agent) + (s.label ? " · " + s.label : "")
}

// What the glance says: { key, arg, state } (the text is tr(key, arg)).
function peekNote(s) {
  if (!s) return null
  var key = s.state === "waiting" ? "agentWaiting" : s.state === "done" ? "agentDone"
    : s.state === "error" ? "agentError" : ""
  return key ? { key: key, arg: sessionLabel(s), state: s.state } : null
}

function summary(list) {
  var out = { working: 0, waiting: 0, done: 0, total: 0 }
  for (var i = 0; i < (list || []).length; i++) {
    out.total++
    if (out[list[i].state] !== undefined) out[list[i].state]++
  }
  return out
}

// The sessions as IPC and Settings show them. The working directory isn't in
// them: it was never kept.
function listing(list, full) {
  return (list || []).map(function(s) {
    var o = { name: s.agent, id: s.id, state: s.state, label: s.label, since: s.since }
    if (full) { o.message = s.message; o.pid = s.pid; o.address = s.address }
    return o
  })
}

// The window to raise for a session, from `hyprctl clients -j` as parsed by
// NotificationModel: its address when that window is still there, else the
// window of its pid; never guessed from a class or a title.
function focusTarget(s, windows) {
  if (!s) return null
  var list = Array.isArray(windows) ? windows : []
  var want = cleanAddress(s.address).toLowerCase()
  if (want !== "") {
    for (var i = 0; i < list.length; i++)
      if (str(list[i].address).toLowerCase() === want) return { focus: list[i].address }
  }
  var w = cleanPid(s.pid) ? NM.windowForPid(list, s.pid) : null
  return w ? { focus: w.address } : null
}

function byteLength(text) {
  return unescape(encodeURIComponent(text)).length
}

// The payload of `island agentJson`: { agent, id, state, cwd, message, pid,
// address }, nothing else, or null when it is too big or not an object.
function parseJson(payload) {
  var text = str(payload)
  if (text === "" || byteLength(text) > MAX_PAYLOAD) return null
  var o
  try { o = JSON.parse(text) } catch (e) { return null }
  if (!o || typeof o !== "object" || Array.isArray(o)) return null
  return {
    agent: cleanName(o.agent), id: cleanId(o.id), state: cleanText(o.state, 24).toLowerCase(),
    cwd: str(o.cwd).slice(0, 1024), message: cleanText(o.message, MAX_MESSAGE),
    pid: cleanPid(o.pid), address: cleanAddress(o.address)
  }
}
