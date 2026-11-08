.pragma library

// What a notification may make the island do. Anything on the session bus can
// send one, so links and commands are checked here before they are used, and
// failing a check means the thing is simply not offered. Same intent as
// Omarchy's notifications Security.js, written for the island's needs.

var MAX_APP = 256
var MAX_SUMMARY = 2048
var MAX_BODY = 32768
var MAX_ACTIONS = 16
var MAX_LABEL = 256
var MAX_URL = 4096
var MAX_ARGV = 32
var MAX_IMAGE_PATH = 1024

function bounded(value, max) {
  return typeof value === "string" ? value.slice(0, max) : (value === undefined || value === null ? "" : String(value).slice(0, max))
}

function hasControlChars(s) { return /[\x00-\x20\x7f-\x9f\\]/.test(s) }

// A plain DNS name with at least one dot. A numeric last label would be read
// as an IPv4 address by a browser (0x7f.1 → 127.0.0.1), so it is refused.
function hostname(raw) {
  if (typeof raw !== "string" || raw.length > 253) return ""
  var h = raw.toLowerCase().replace(/\.$/, "")
  if (!/^[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?(?:\.[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?)+$/.test(h)) return ""
  if (/^(?:0x[0-9a-f]*|[0-9]+)$/i.test(h.split(".").pop())) return ""
  return h
}

// http(s) only, no user info, no encoded control characters or quotes.
// Returns the URL to open, or "" when it isn't safe to hand to a browser.
function safeHttpUrl(raw) {
  if (typeof raw !== "string" || raw.length > MAX_URL || hasControlChars(raw)) return ""
  if (/%(?:0[0-9a-f]|1[0-9a-f]|7f|25|5c)/i.test(raw) || /[<>"']/.test(raw)) return ""
  var m = raw.match(/^(https?):\/\/([^/?#]+)([\s\S]*)$/i)
  if (!m || /^[a-z]+:\/\/[^/?#]*@/i.test(raw)) return ""
  var a = m[2].match(/^([^:%@]+)(?::([0-9]{1,5}))?$/)
  if (!a) return ""
  var h = hostname(a[1])
  if (!h || (a[2] && (+a[2] < 1 || +a[2] > 65535))) return ""
  if (/%(?![0-9a-f]{2})/i.test(m[3])) return ""
  return m[1].toLowerCase() + "://" + h + (a[2] ? ":" + String(+a[2]) : "") + (m[3] || "/")
}

// The first link in a body (an <a href> or a bare http(s) URL) that passes
// safeHttpUrl, or "".
function firstLink(text) {
  var s = String(text || "").slice(0, MAX_BODY)
  var re = /href=["']([^"']+)["']|(https?:\/\/[^\s<>"']+)/gi
  var m
  var guard = 0
  while ((m = re.exec(s)) !== null && guard++ < 20) {
    var raw = (m[1] || m[2] || "").replace(/&amp;/g, "&").replace(/[).,;:!?]+$/, "")
    var safe = safeHttpUrl(raw)
    if (safe) return safe
  }
  return ""
}

// The omarchy-exec-argv hint (omarchy-notification-send --exec): a JSON array
// of strings, run as an argv, never through a shell. Fails closed.
function parseExecArgv(value) {
  if (Array.isArray(value)) {
    try { value = JSON.stringify(value) } catch (e) { return null }
  }
  var text = String(value || "")
  if (!text || text.length > MAX_BODY) return null
  var parsed
  try { parsed = JSON.parse(text) } catch (e) { return null }
  if (!Array.isArray(parsed) || parsed.length === 0 || parsed.length > MAX_ARGV) return null
  for (var i = 0; i < parsed.length; i++) {
    if (typeof parsed[i] !== "string" || parsed[i].length > MAX_URL || parsed[i].indexOf("\u0000") >= 0) return null
  }
  if (!parsed[0] || parsed[0].charAt(0) === "-") return null
  return parsed
}

// A picture the sender named on disk (image-path hint, image://icon//path or
// file://). Kernel interfaces and other services' runtime state are refused
// by their spelling; one spelling per file (no "." or ".." segments).
function localImagePath(raw) {
  var text = typeof raw === "string" && raw.length <= MAX_IMAGE_PATH * 3 ? raw : ""
  if (text.indexOf("image://icon/") === 0) text = text.slice("image://icon/".length)
  if (/^file:\/\//i.test(text)) {
    if (!/^file:\/\/\//i.test(text) || /[?#]/.test(text)) return ""
    try { text = decodeURIComponent(text.slice("file://".length)) } catch (e) { return "" }
  }
  if (!text || text.length > MAX_IMAGE_PATH || text.charAt(0) !== "/" || /[\x00-\x1f\x7f-\x9f]/.test(text)) return ""
  var parts = text.slice(1).split("/")
  for (var i = 0; i < parts.length; i++)
    if (parts[i] === "" || parts[i] === "." || parts[i] === "..") return ""
  if (/^\/(?:proc|sys|dev)(?:\/|$)/.test(text)) return ""
  if (/^\/(?:var\/)?run(?:\/|$)/.test(text) && !/^\/run\/user\/[0-9]+\/./.test(text)) return ""
  return text
}

function localImageUrl(raw) {
  var path = localImagePath(raw)
  return path ? "file://" + path.split("/").map(encodeURIComponent).join("/") : ""
}

// An icon-theme name as senders spell them ("firefox", "org.telegram.desktop").
function iconName(raw) {
  var s = String(raw || "")
  return /^[A-Za-z0-9_][A-Za-z0-9_.+-]{0,255}$/.test(s) ? s : ""
}
