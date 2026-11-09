import { test } from "node:test"
import assert from "node:assert/strict"
import { load, plain } from "./load.mjs"

const NM = load("NotificationModel.js")

function note(over) {
  return Object.assign({
    id: 7, appName: "Mail", appIcon: "mail-unread", desktopEntry: "", image: "",
    summary: "Hello", body: "Body", urgency: 1, expireTimeout: 0,
    actions: [], hints: {}, resident: false, transient: false,
  }, over)
}

test("plainText: tags out, entities decoded, whitespace folded", () => {
  assert.equal(NM.plainText("<b>Hi</b>&nbsp;&amp;<br>there   you"), "Hi &\nthere you")
  assert.equal(NM.plainText("&#60;x&#62;"), "<x>")
  assert.equal(NM.plainText("&#7;"), "")
})

test("snapshot: actions, default, inline reply and settings are split out", () => {
  const row = NM.snapshot(note({
    actions: [
      { identifier: "default", text: "" },
      { identifier: "inline-reply", text: "Reply" },
      { identifier: "settings", text: "Settings" },
      { identifier: "archive", text: " Archive " },
    ],
    body: 'see <a href="https://example.com/x">x</a>',
  }), "k1", 1000)
  assert.equal(row.hasDefault, true)
  assert.deepEqual(plain(row.actions), [{ id: "archive", text: "Archive" }])
  assert.equal(row.link, "https://example.com/x")
  assert.equal(row.body, "see x")
  assert.equal(row.live, true)
  assert.equal(row.read, false)
})

test("snapshot: transient hint and bad exec argv", () => {
  const row = NM.snapshot(note({ hints: { transient: true, "omarchy-exec-argv": "rm -rf /" } }), "k", 0)
  assert.equal(row.transient, true)
  assert.equal(row.execArgv, null)
})

test("peekFor: critical stays, sender timeout clamped", () => {
  assert.equal(NM.peekFor({ urgency: 2, expire: 0 }, 3500), 0)
  assert.equal(NM.peekFor({ urgency: 1, expire: 100 }, 3500), 1600)
  assert.equal(NM.peekFor({ urgency: 1, expire: 99999 }, 3500), 30000)
  assert.equal(NM.peekFor({ urgency: 1, expire: 0 }, 3500), 3500)
})

test("bypassesQuiet: Omarchy toasts and bare notify-send critical only", () => {
  assert.equal(NM.bypassesQuiet({ app: "omarchy-action", urgency: 1 }), true)
  assert.equal(NM.bypassesQuiet({ app: "notify-send", urgency: 2 }), true)
  assert.equal(NM.bypassesQuiet({ app: "Slack", urgency: 2 }), false)
})

test("groupByApp: groups in arrival order with unread and critical", () => {
  const groups = NM.groupByApp([
    { app: "A", read: false, urgency: 1 },
    { app: "B", read: true, urgency: 2 },
    { app: "A", read: true, urgency: 1 },
  ])
  assert.deepEqual(plain(groups.map(g => [g.app, g.rows.length, g.unread, g.critical])), [["A", 2, 1, false], ["B", 1, 0, true]])
})

test("prune: live stays, transient and old closed rows go, limit applies", () => {
  const now = 10 * 24 * 3600 * 1000
  const rows = [
    { live: true, time: 0 },
    { live: false, transient: true, time: now },
    { live: false, time: now - 25 * 3600 * 1000 },
    { live: false, time: now, n: 1 },
    { live: false, time: now, n: 2 },
  ]
  const out = NM.prune(rows, 1, now)
  assert.equal(out.length, 2)
  assert.equal(out[0].live, true)
  assert.equal(out[1].n, 1)
})

test("serialise → parseStore keeps only safe stored fields", () => {
  const row = NM.snapshot(note({ hints: { "omarchy-exec-argv": '["x"]' }, actions: [{ identifier: "a", text: "A" }] }), "k", 5)
  const back = NM.parseStore(NM.serialise([row]), 0)
  assert.equal(back.length, 1)
  assert.equal(back[0].summary, "Hello")
  assert.equal(back[0].execArgv, null)
  assert.deepEqual(plain(back[0].actions), [])
  assert.equal(back[0].live, false)
  assert.deepEqual(plain(NM.parseStore("not json", 0)), [])
})

test("relativeTime buckets", () => {
  assert.equal(NM.relativeTime(0, 30000).key, "nt_now")
  assert.deepEqual(plain(NM.relativeTime(0, 5 * 60000)), { key: "nt_minAgo", arg: 5 })
  assert.deepEqual(plain(NM.relativeTime(0, 3 * 3600000)), { key: "nt_hAgo", arg: 3 })
  assert.equal(NM.relativeTime(0, 2 * 86400000).key, "")
})
