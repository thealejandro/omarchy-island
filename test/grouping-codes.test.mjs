import { test } from "node:test"
import assert from "node:assert/strict"
import { load, plain } from "./load.mjs"

const NM = load("NotificationModel.js")
const IM = load("IslandModel.js")

const codes = text => plain(NM.codes(text))

function note(over) {
  return Object.assign({
    id: 1, appName: "Chromium", appIcon: "chromium", desktopEntry: "", image: "",
    summary: "", body: "", urgency: 1, expireTimeout: 0,
    actions: [], hints: {}, resident: false, transient: false,
  }, over)
}

test("codes: only next to a code word", () => {
  assert.deepEqual(codes("Your verification code is 482913"), ["482913"])
  assert.deepEqual(codes("Tu código de acceso: 482 913"), ["482913"])
  assert.deepEqual(codes("G-482913 is your Google verification code."), ["482913"])
  assert.deepEqual(codes("2FA: A9F3K2"), ["A9F3K2"])
  assert.deepEqual(codes("Order 482913 has shipped"), [])
  assert.deepEqual(codes("412 tests passed"), [])
})

test("codes: product names, years and long numbers are not codes", () => {
  assert.deepEqual(codes("Claude Code 2.1.40 is out, build 48291"), [])
  assert.deepEqual(codes("VS Code 1234 update"), [])
  assert.deepEqual(codes("Confirmation for 2026 tickets"), [])
  assert.deepEqual(codes("Your code is 1234567890123"), [])
  assert.deepEqual(codes("use code SAVE20 today"), ["SAVE20"])
})

test("liftSource: a browser's leading self-link becomes the source", () => {
  const lifted = NM.liftSource('<a href="https://web.whatsapp.com/">web.whatsapp.com</a>\n\nSam: lunch?')
  assert.equal(lifted.source, "web.whatsapp.com")
  assert.equal(lifted.body, "Sam: lunch?")
  // A real link the sender put first stays where it is.
  const kept = NM.liftSource('<a href="https://example.com/x">Read the post</a> now')
  assert.equal(kept.source, "")
  // An unsafe href never names a site.
  assert.equal(NM.liftSource('<a href="javascript:x">evil.com</a> hi').source, "")
})

test("snapshot: browser rows carry their site and codes, other apps don't lift", () => {
  const row = NM.snapshot(note({
    summary: "GitHub",
    body: '<a href="https://github.com/">github.com</a> Your verification code is 552 019',
  }), "k1", 0)
  assert.equal(row.source, "github.com")
  assert.equal(row.body, "Your verification code is 552 019")
  assert.deepEqual(plain(row.codes), ["552019"])
  assert.equal(NM.senderLabel(row), "github.com")

  const mail = NM.snapshot(note({ appName: "Thunderbird", body: '<a href="https://github.com/">github.com</a> hi' }), "k2", 0)
  assert.equal(mail.source, "")
  assert.equal(NM.senderLabel(mail), "Thunderbird")
})

test("groupByApp: per site for browsers, toasts alone, apps together", () => {
  const rows = [
    { key: "1", app: "Chromium", source: "web.whatsapp.com", read: false, urgency: 1 },
    { key: "2", app: "Chromium", source: "github.com", read: false, urgency: 1 },
    { key: "3", app: "Chromium", source: "web.whatsapp.com", read: true, urgency: 1 },
    { key: "4", app: "omarchy-action", source: "", read: false, urgency: 1 },
    { key: "5", app: "omarchy-action", source: "", read: false, urgency: 1 },
    { key: "6", app: "Slack", source: "", read: false, urgency: 1 },
    { key: "7", app: "Slack", source: "", read: false, urgency: 1 },
  ]
  const groups = NM.groupByApp(rows)
  assert.deepEqual(plain(groups.map(g => [g.key, g.app, g.rows.length])), [
    ["web:web.whatsapp.com", "web.whatsapp.com", 2],
    ["web:github.com", "github.com", 1],
    ["toast:4", "omarchy-action", 1],
    ["toast:5", "omarchy-action", 1],
    ["app:slack", "Slack", 2],
  ])
})

test("history keeps the site but never the codes", () => {
  const row = NM.snapshot(note({ body: '<a href="https://github.com/">github.com</a> code 552019' }), "k", 0)
  const text = NM.serialise([row])
  assert.equal(JSON.parse(text).rows[0].codes, undefined)
  const back = NM.parseStore(text, 0)[0]
  assert.equal(back.source, "github.com")
  assert.deepEqual(plain(back.codes), [])
  assert.equal(NM.parseStore(JSON.stringify({ version: 1, rows: [{ summary: "x", source: "127.0.0.1" }] }), 0)[0].source, "")
})

test("strings: Spanish has every English key", () => {
  const missing = Object.keys(IM.STRINGS.en).filter(k => !(k in IM.STRINGS.es))
  assert.deepEqual(plain(missing), [])
})
