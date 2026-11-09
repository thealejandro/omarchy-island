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

// ------------------------------------------------------------- who and what
// Modelled on Omarchy's notifications plugin (Markup.js, Detect.js; MIT).

// Only a browser's leading anchor names a site; any other app's is a link.
var BROWSERS = /^(google chrome|chromium|brave|brave browser|brave origin|microsoft edge|vivaldi|opera|helium|firefox|zen|librewolf)$/i
function isBrowser(app) { return BROWSERS.test(str(app).trim()) }

function hostOf(url) {
  var safe = Sec.safeHttpUrl(str(url).replace(/&amp;/g, "&"))
  var m = /^https?:\/\/([^\/:?#]+)/.exec(safe)
  return m ? Sec.hostname(m[1]) : ""
}

// A browser body that opens with a link to the site itself ("<a
// href=https://web.whatsapp.com/>web.whatsapp.com</a>") is the site naming
// itself: lift it off as the source. The label must be that host or a bare
// domain, so a real link the sender put first stays.
function liftSource(rawBody) {
  var text = str(rawBody)
  var m = /^\s*<a\s+href=["']?([^"'\s>]+)["']?[^>]*>([\s\S]*?)<\/a>\s*/i.exec(text)
  if (!m) return { source: "", body: text }
  var host = hostOf(m[1])
  var label = m[2].trim()
  if (!host || !(label === host || /^[\w.-]+\.[a-z]{2,}$/i.test(label))) return { source: "", body: text }
  return { source: host, body: text.slice(m[0].length) }
}

function slug(text) {
  return str(text).toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "")
}

// Which group a row goes in: per site for a browser, alone for Omarchy's own
// toasts and bare notify-send (they are unrelated messages), else per app.
function groupKey(row) {
  if (!row) return ""
  if (row.app === "omarchy-action" || row.app === "notify-send" || row.app === "") return "toast:" + str(row.key)
  if (row.source && isBrowser(row.app)) return "web:" + row.source
  return "app:" + slug(row.app)
}

// The sender's name as shown: the site for a browser, else the app.
function senderLabel(row) {
  if (!row) return ""
  return row.source && isBrowser(row.app) ? row.source : str(row.app)
}

// A code is only a code if the text says so: by shape alone it is also an
// order number, a port, a build or a year. It must sit near its keyword
// (English or Spanish), so "Claude Code 2.1.40" offers nothing.
var CODE_WORDS = /\b(code|otp|one[\s-]?time|verification|verify|passcode|pin|2fa|two[\s-]factor|security key|confirm(?:ation)?|c[oó]digo|verificaci[oó]n|clave|contrase[nñ]a de un solo uso)\b/i
var CODE_SHAPE = /(?:^|[^\w-])((?:[A-Za-z]-)?\d{2,4}[\s-]\d{2,5}|(?:[A-Za-z]-)?\d{4,8})(?![\w-])/g
var CODE_ALNUM = /(?:^|[^\w-])([A-Z0-9]{5,8})(?![\w-])/g
var NEAR_BEFORE = 24
var NEAR_AFTER = 72
var MAX_CODES = 4

// "Claude Code", "VS Code": a capitalised word before a capitalised "Code"
// is a product name; "use code SAVE20" and "Your Verification Code" are not.
function isProductName(t, at, word) {
  if (word.toLowerCase() !== "code" || word[0] !== "C") return false
  var before = /([A-Za-z][\w.]*)\s+$/.exec(t.slice(Math.max(0, at - 20), at))
  if (!before || !/^([A-Z][a-z]+|[A-Z]{2,})$/.test(before[1])) return false
  return !CODE_WORDS.test(before[1])
}

function harvest(t, re, seen, out, keep) {
  var m
  re.lastIndex = 0
  while ((m = re.exec(t)) !== null) {
    var value = keep(m[1])
    if (!value || seen[value]) continue
    seen[value] = true
    out.push(value)
  }
}

// One-time codes in a notification's text, digits first; [] when unsure.
function codes(text) {
  var t = str(text).slice(0, Sec.MAX_BODY)
  var spans = [], m
  var re = new RegExp(CODE_WORDS.source, "gi")
  while ((m = re.exec(t)) !== null && spans.length < MAX_CODES) {
    if (!isProductName(t, m.index, m[0]))
      spans.push([Math.max(0, m.index - NEAR_BEFORE), m.index + m[0].length + NEAR_AFTER])
  }
  var out = [], seen = {}, i
  for (i = 0; i < spans.length; i++) {
    harvest(t.slice(spans[i][0], spans[i][1]), CODE_SHAPE, seen, out, function(raw) {
      var v = raw.replace(/^[A-Za-z]-/, "").replace(/[\s-]/g, "")
      if (v.length < 4 || v.length > 8 || /^(19|20)\d\d$/.test(v)) return ""
      return v
    })
  }
  if (out.length) return out.slice(0, MAX_CODES)
  for (i = 0; i < spans.length; i++) {
    harvest(t.slice(spans[i][0], spans[i][1]), CODE_ALNUM, seen, out, function(raw) {
      return /\d/.test(raw) && /[A-Z]/.test(raw) ? raw : ""
    })
  }
  return out.slice(0, MAX_CODES)
}

// -------------------------------------------------------- whose window
// Clicking a notification with nothing of its own to do goes to the
// sender: its window, else its site, else its link. Modelled on Omarchy's
// notifications plugin (Service.qml; MIT).

var BROWSER_CLASSES = /^(chrome|google-chrome|chromium|firefox|zen|brave|microsoft-edge|edge|vivaldi|helium|librewolf)/
// Every Chrome title ends "- Google Chrome"; left on, google.com would match
// every Chrome window.
var BROWSER_SUFFIX = /\s*[-–—|]\s*(google chrome|chromium|mozilla firefox|firefox|zen browser|brave origin|brave|microsoft edge|vivaldi|helium|librewolf)\s*$/
// Labels that name nobody: common subdomains and public suffixes.
var GENERIC_LABELS = { www: 1, app: 1, web: 1, my: 1, m: 1, mail: 1, com: 1, org: 1, net: 1,
                       io: 1, co: 1, dev: 1, ai: 1, so: 1, site: 1, uk: 1 }

// `hyprctl clients -j` → [{ wmClass, title, address, pid, focusOrder }].
// focusOrder 0 is the window you are in, 1 the one before it.
function parseClients(text) {
  var list
  try { list = JSON.parse(str(text)) } catch (e) { return [] }
  if (!Array.isArray(list)) return []
  var out = []
  for (var i = 0; i < list.length && out.length < 500; i++) {
    var c = list[i]
    if (!c || typeof c !== "object") continue
    var address = str(c.address)
    var wmClass = str(c["class"])
    if (!/^0x[0-9a-f]{1,16}$/i.test(address) || wmClass === "") continue
    out.push({
      wmClass: wmClass.slice(0, 256),
      title: str(c.title).slice(0, 1024),
      address: address,
      pid: Number(c.pid) || 0,
      focusOrder: c.focusHistoryID === undefined ? 9999 : Number(c.focusHistoryID)
    })
  }
  return out
}

function mostRecent(found) {
  var best = null
  for (var i = 0; i < found.length; i++)
    if (!best || found[i].focusOrder < best.focusOrder) best = found[i]
  return best
}

function wordIn(text, word) {
  var at = text.indexOf(word)
  while (at >= 0) {
    var before = at === 0 ? "" : text.charAt(at - 1)
    var after = text.charAt(at + word.length)
    if (!/[a-z0-9]/.test(before) && !/[a-z0-9]/.test(after)) return true
    at = text.indexOf(word, at + 1)
  }
  return false
}

// What names a site in a title, most telling first: "app.slack.com" gives
// ["slack"], "news.ycombinator.com" ["news", "ycombinator"].
function brandsOf(host) {
  var labels = str(host).toLowerCase().split(".")
  var out = []
  for (var i = 0; i < labels.length; i++)
    if (labels[i].length >= 4 && !GENERIC_LABELS[labels[i]]) { out.push(labels[i]); break }
  var registered = labels.length >= 2 ? labels[labels.length - 2] : ""
  if (registered.length >= 4 && !GENERIC_LABELS[registered] && out.indexOf(registered) < 0) out.push(registered)
  return out
}

// The sender process's own window: the one identity that is never a guess.
function windowForPid(windows, pid) {
  var want = Number(pid) || 0
  if (!want) return null
  var found = []
  for (var i = 0; i < windows.length; i++) if (windows[i].pid === want) found.push(windows[i])
  return mostRecent(found)
}

// A window for a site ("web.whatsapp.com": a web app's class, else a browser
// tab whose title names it) or an app ("Telegram Desktop": its class).
function windowForSource(windows, source) {
  var name = str(source).toLowerCase()
  if (!name) return null
  var found = [], i
  if (name.indexOf(".") > 0) {
    for (i = 0; i < windows.length; i++)
      if (windows[i].wmClass.toLowerCase().indexOf(name) >= 0) found.push(windows[i])
    if (found.length) return mostRecent(found)
    // Browsers only: "slack" in an editor's title is a file name.
    var brands = brandsOf(name)
    for (var b = 0; b < brands.length; b++) {
      found = []
      for (i = 0; i < windows.length; i++) {
        if (!BROWSER_CLASSES.test(windows[i].wmClass.toLowerCase())) continue
        if (wordIn(windows[i].title.toLowerCase().replace(BROWSER_SUFFIX, ""), brands[b])) found.push(windows[i])
      }
      if (found.length) return mostRecent(found)
    }
    return null
  }
  // An app name: whole class, a class ending in its words ("Telegram
  // Desktop" is org.telegram.desktop) or in its name, or a Chrome web app's
  // host label.
  // Never shorter than four characters.
  var slugged = name.replace(/[^a-z0-9]+/g, "")
  if (slugged.length < 4) return null
  var words = name.split(/[^a-z0-9]+/).filter(function(w) { return w })
  var tail = words.length > 1 ? "." + words.join(".") : ""
  for (i = 0; i < windows.length; i++) {
    var lower = windows[i].wmClass.toLowerCase()
    if (lower === slugged) { found.push(windows[i]); continue }
    var dotted = "." + lower.replace(/[^a-z0-9]+/g, ".")
    if (tail && dotted.slice(-tail.length) === tail) { found.push(windows[i]); continue }
    // Reverse-DNS classes: "Ghostty" is com.mitchellh.ghostty.
    if (dotted.slice(-(slugged.length + 1)) === "." + slugged) { found.push(windows[i]); continue }
    if (lower.indexOf("chrome-") !== 0) continue
    var labels = lower.substring(7).split("__")[0].split(".")
    if (labels.indexOf(slugged) >= 0) found.push(windows[i])
  }
  return mostRecent(found)
}

// Where a click goes when the sender gave nothing to run: { focus: address },
// { open: url } or null. For a browser's site: the tab that names it, else
// the site's front page (the browser's pid is any of its windows, which says
// nothing about the site). For an app: its own window, by pid or by class.
// Then the link in the text: a Slack message quoting a link is still
// Slack's, so the window wins over the link.
function clickTarget(row, windows) {
  if (!row) return null
  var list = windows || []
  var site = row.source && isBrowser(row.app) ? row.source : ""
  var win = site ? windowForSource(list, site) : (windowForPid(list, row.pid) || windowForSource(list, row.app))
  if (win) return { focus: win.address }
  if (site && Sec.hostname(site)) return { open: "https://" + Sec.hostname(site) + "/" }
  var link = Sec.safeHttpUrl(str(row.link))
  return link ? { open: link } : null
}

// Whether clickTarget could find something without looking at windows.
function mayHaveTarget(row) {
  return !!row && (!!row.source || !!row.link || Number(row.pid) > 0 || str(row.app).replace(/[^a-z0-9]/gi, "").length >= 4)
}

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
  var app = Sec.bounded(n.appName, Sec.MAX_APP).trim()
  var lifted = isBrowser(app) ? liftSource(Sec.bounded(n.body, Sec.MAX_BODY)) : { source: "", body: Sec.bounded(n.body, Sec.MAX_BODY) }
  var rawBody = lifted.body
  var summary = Sec.bounded(n.summary, Sec.MAX_SUMMARY).trim()
  var body = plainText(rawBody)
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
    // The sending process (libnotify's hint), to find its window; never stored.
    pid: Math.max(0, parseInt(hint(n, "sender-pid"), 10) || 0),
    app: app,
    source: lifted.source,
    appIcon: iconFile ? "" : Sec.iconName(appIcon.replace(/^image:\/\/icon\//, "")),
    iconFile: iconFile,
    desktopEntry: Sec.iconName(n.desktopEntry),
    image: image,
    summary: summary,
    body: body,
    link: Sec.firstLink(rawBody),
    // Only while live, and never written to disk.
    codes: codes(summary + "\n" + body),
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

// Newest first in, groups out in the order their newest row arrived. A
// browser's rows are grouped per site, Omarchy's toasts stand alone.
function groupByApp(rows) {
  var groups = []
  var at = {}
  for (var i = 0; i < rows.length; i++) {
    var r = rows[i]
    var key = groupKey(r)
    if (at[key] === undefined) {
      at[key] = groups.length
      groups.push({ key: key, app: senderLabel(r) || "?", rows: [], unread: 0, critical: false })
    }
    var g = groups[at[key]]
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

var STORED = ["app", "source", "appIcon", "iconFile", "desktopEntry", "summary", "body", "link", "urgency", "time", "read"]

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
      pid: 0,
      app: Sec.bounded(s.app, Sec.MAX_APP),
      source: Sec.hostname(str(s.source)),
      codes: [],
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
