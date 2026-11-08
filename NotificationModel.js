.pragma library
.import "NotificationSecurity.js" as Sec

// Plain data for the island's notifications: the row a live notification is
// turned into, how long it peeks, how rows are grouped, pruned and stored.
// No QML objects are kept in a row, so a row outlives its notification and
// can be written to disk as it is.

var LOW = 0
var NORMAL = 1
var CRITICAL = 2
var LIVE_LIMIT = 100
var RETENTION_MS = 24 * 3600 * 1000
var STORE_VERSION = 1

function str(v) { return v === undefined || v === null ? "" : String(v) }

// Markup → plain text: tags out, the common entities decoded, whitespace
// folded. Bodies are always drawn as plain text.
function plainText(text) {
  var s = str(text).slice(0, Sec.MAX_BODY)
  s = s.replace(/<br\s*\/?>/gi, "\n").replace(/<\/p>/gi, "\n").replace(/<[^>]*>/g, "")
  s = s.replace(/&lt;/g, "<").replace(/&gt;/g, ">").replace(/&quot;/g, "\"")
    .replace(/&#39;|&apos;/g, "'").replace(/&nbsp;/g, " ")
    .replace(/&#(\d{1,6});/g, function(_, n) { var c = parseInt(n, 10); return c > 31 && c < 0x110000 ? String.fromCodePoint(c) : "" })
    .replace(/&amp;/g, "&")
  return s.replace(/[ \t]+/g, " ").replace(/\n{3,}/g, "\n\n").trim()
}

function oneLine(text) { return str(text).replace(/\s*\n\s*/g, "  ").trim() }

function hint(n, name) {
  try { var h = n.hints; return h && h[name] !== undefined ? h[name] : undefined } catch (e) { return undefined }
}

// A Quickshell Notification → row. `key` is the island's own id for it.
function snapshot(n, key, now) {
  var actions = []
  var hasDefault = false
  var list = n.actions || []
  for (var i = 0; i < list.length && actions.length < Sec.MAX_ACTIONS; i++) {
    var id = Sec.bounded(list[i].identifier, Sec.MAX_LABEL)
    if (id === "default") { hasDefault = true; continue }
    if (id === "inline-reply" || id === "settings") continue
    var text = Sec.bounded(list[i].text, Sec.MAX_LABEL).trim()
    if (text !== "") actions.push({ id: id, text: text })
  }
  var rawBody = Sec.bounded(n.body, Sec.MAX_BODY)
  var image = str(n.image)
  // A picture handed over the bus only lives as long as the notification.
  image = image.indexOf("image://qsimage/") === 0 ? image : Sec.localImageUrl(image)
  var appIcon = str(n.appIcon)
  var iconFile = Sec.localImageUrl(appIcon)
  var argv = Sec.parseExecArgv(hint(n, "omarchy-exec-argv"))
  var hasReply = false, placeholder = ""
  try { hasReply = n.hasInlineReply === true; placeholder = Sec.bounded(n.inlineReplyPlaceholder, 128) } catch (e) { }
  return {
    key: key,
    id: Number(n.id) || 0,
    app: Sec.bounded(n.appName, Sec.MAX_APP).trim(),
    appIcon: iconFile ? "" : Sec.iconName(appIcon.replace(/^image:\/\/icon\//, "")),
    iconFile: iconFile,
    desktopEntry: Sec.iconName(n.desktopEntry),
    image: image,
    summary: Sec.bounded(n.summary, Sec.MAX_SUMMARY).trim(),
    body: plainText(rawBody),
    link: Sec.firstLink(rawBody),
    urgency: Number(n.urgency) === CRITICAL ? CRITICAL : (Number(n.urgency) === LOW ? LOW : NORMAL),
    expire: Number(n.expireTimeout) || 0,
    actions: actions,
    hasDefault: hasDefault,
    execArgv: argv,
    reply: hasReply,
    replyPlaceholder: placeholder,
    resident: n.resident === true,
    transient: n.transient === true || hint(n, "transient") === true,
    time: now,
    read: false,
    live: true
  }
}

// The same row after the notification changed in place (replaces_id).
function refreshed(old, fresh) {
  var row = fresh
  row.key = old.key
  row.read = false
  return row
}

// How long the glance stays: critical until dealt with, otherwise the
// island's setting, or the sender's own timeout within 1.6–30 s.
function peekFor(row, base) {
  if (!row) return base
  if (row.urgency === CRITICAL) return 0
  if (row.expire > 0) return Math.max(1600, Math.min(30000, row.expire))
  return base
}

// Like Omarchy: its own action toasts and a bare notify-send critical get
// through Do Not Disturb; chat apps marking everything critical do not.
function bypassesQuiet(row) {
  if (!row) return false
  if (row.app === "omarchy-action") return true
  return row.app === "notify-send" && row.urgency === CRITICAL
}

function appLabel(row) { return row && row.app ? row.app : "" }

// Newest first in, groups out in the order their newest row arrived.
function groupByApp(rows) {
  var groups = []
  var at = {}
  for (var i = 0; i < rows.length; i++) {
    var r = rows[i]
    var name = appLabel(r) || "?"
    if (at[name] === undefined) {
      at[name] = groups.length
      groups.push({ app: name, rows: [], unread: 0, critical: false })
    }
    var g = groups[at[name]]
    g.rows.push(r)
    if (!r.read) g.unread++
    if (r.urgency === CRITICAL) g.critical = true
  }
  return groups
}

// Live rows always stay; closed ones go after RETENTION_MS or beyond `limit`
// rows in all. Transient rows are never kept once closed.
function prune(rows, limit, now) {
  var out = []
  var closed = 0
  for (var i = 0; i < rows.length; i++) {
    var r = rows[i]
    if (r.live) { out.push(r); continue }
    if (r.transient) continue
    if (now - Number(r.time || 0) > RETENTION_MS) continue
    if (closed >= limit) continue
    closed++
    out.push(r)
  }
  return out
}

var STORED = ["app", "appIcon", "iconFile", "desktopEntry", "summary", "body", "link", "urgency", "time", "read"]

// What goes to disk: closed rows (a live one is stored as closed, since
// its sender's object can't be brought back), without the bus picture,
// actions or commands.
function serialise(rows) {
  var out = []
  for (var i = 0; i < rows.length; i++) {
    var r = rows[i]
    if (r.transient) continue
    var s = { }
    for (var k = 0; k < STORED.length; k++) s[STORED[k]] = r[STORED[k]]
    if (r.image && r.image.indexOf("file://") === 0) s.image = r.image
    out.push(s)
  }
  return JSON.stringify({ version: STORE_VERSION, rows: out }) + "\n"
}

function parseStore(text, seedStart) {
  var data
  try { data = JSON.parse(str(text)) } catch (e) { return [] }
  var list = data && Array.isArray(data.rows) ? data.rows : []
  var out = []
  for (var i = 0; i < list.length && out.length < 1000; i++) {
    var s = list[i]
    if (!s || typeof s !== "object") continue
    var summary = Sec.bounded(s.summary, Sec.MAX_SUMMARY)
    var body = Sec.bounded(s.body, Sec.MAX_BODY)
    if (summary === "" && body === "") continue
    out.push({
      key: "h" + (seedStart + i),
      id: 0,
      app: Sec.bounded(s.app, Sec.MAX_APP),
      appIcon: Sec.iconName(s.appIcon),
      iconFile: Sec.localImageUrl(str(s.iconFile)),
      desktopEntry: Sec.iconName(s.desktopEntry),
      image: Sec.localImageUrl(str(s.image)),
      summary: summary,
      body: body,
      link: Sec.safeHttpUrl(str(s.link)),
      urgency: Number(s.urgency) === CRITICAL ? CRITICAL : (Number(s.urgency) === LOW ? LOW : NORMAL),
      expire: 0, actions: [], hasDefault: false, execArgv: null, reply: false, replyPlaceholder: "",
      resident: false, transient: false,
      time: Number(s.time) || 0,
      read: s.read === true,
      live: false
    })
  }
  return out
}

// { key, arg } for the island's strings: nt_now, nt_minAgo, nt_hAgo, or a
// date for anything older than a day (arg is the Date).
function relativeTime(ts, now) {
  var d = Math.max(0, Number(now) - Number(ts))
  if (d < 60000) return { key: "nt_now", arg: undefined }
  if (d < 3600000) return { key: "nt_minAgo", arg: Math.floor(d / 60000) }
  if (d < 86400000) return { key: "nt_hAgo", arg: Math.floor(d / 3600000) }
  return { key: "", arg: new Date(Number(ts)) }
}
