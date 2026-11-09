import { test } from "node:test"
import assert from "node:assert/strict"
import { load, plain } from "./load.mjs"

const NM = load("NotificationModel.js")

const clients = JSON.stringify([
  { class: "chromium", title: "WhatsApp - Chromium", address: "0xa1", pid: 100, focusHistoryID: 3 },
  { class: "chromium", title: "(2) WhatsApp - Chromium", address: "0xa2", pid: 100, focusHistoryID: 1 },
  { class: "chromium", title: "Pull requests · GitHub - Chromium", address: "0xa3", pid: 100, focusHistoryID: 2 },
  { class: "chrome-app.slack.com__-Default", title: "Slack", address: "0xb1", pid: 200, focusHistoryID: 4 },
  { class: "org.telegram.desktop", title: "Telegram", address: "0xc1", pid: 300, focusHistoryID: 5 },
  { class: "dev.zed.Zed", title: "slack.md — notes", address: "0xd1", pid: 400, focusHistoryID: 0 },
  { class: "evil", title: "x", address: "0xe1\"); os.exit(", pid: 500 },
  { class: "", title: "no class", address: "0xf1", pid: 600 },
])
const windows = NM.parseClients(clients)

test("parseClients: bad addresses and classless windows are dropped", () => {
  assert.deepEqual(plain(windows.map(w => w.address)), ["0xa1", "0xa2", "0xa3", "0xb1", "0xc1", "0xd1"])
  assert.deepEqual(plain(NM.parseClients("nope")), [])
  assert.deepEqual(plain(NM.parseClients("{}")), [])
})

test("browser site: the most recent tab that names it", () => {
  const row = { app: "Chromium", source: "web.whatsapp.com", pid: 100 }
  assert.deepEqual(plain(NM.clickTarget(row, windows)), { focus: "0xa2" })
  assert.deepEqual(plain(NM.clickTarget({ app: "Chromium", source: "github.com" }, windows)), { focus: "0xa3" })
})

test("browser site with no tab: its front page, not any browser window", () => {
  const row = { app: "Chromium", source: "mail.proton.me", pid: 100, link: "https://example.com/x" }
  assert.deepEqual(plain(NM.clickTarget(row, windows)), { open: "https://mail.proton.me/" })
})

test("an app's pid picks its most recent window", () => {
  assert.deepEqual(plain(NM.clickTarget({ app: "Chromium", pid: 100 }, windows)), { focus: "0xa2" })
})

test("apps: own pid first, then a class that names the app", () => {
  assert.deepEqual(plain(NM.clickTarget({ app: "Telegram Desktop", pid: 300 }, windows)), { focus: "0xc1" })
  assert.deepEqual(plain(NM.clickTarget({ app: "Telegram Desktop" }, windows)), { focus: "0xc1" })
  assert.deepEqual(plain(NM.clickTarget({ app: "Slack" }, windows)), { focus: "0xb1" })
})

test("a file named like the app in an editor's title is no match", () => {
  const onlyEditor = windows.filter(w => w.address === "0xd1")
  assert.equal(NM.clickTarget({ app: "Slack" }, onlyEditor), null)
  assert.equal(NM.clickTarget({ app: "Chromium", source: "app.slack.com" }, onlyEditor).open, "https://app.slack.com/")
})

test("no window: the link, if safe; else nothing", () => {
  assert.deepEqual(plain(NM.clickTarget({ app: "Mail", link: "https://example.com/a" }, [])), { open: "https://example.com/a" })
  assert.equal(NM.clickTarget({ app: "Mail", link: "javascript:alert(1)" }, []), null)
  assert.equal(NM.clickTarget({ app: "ab" }, windows), null)
})

test("mayHaveTarget", () => {
  assert.equal(NM.mayHaveTarget({ app: "ab" }), false)
  assert.equal(NM.mayHaveTarget({ app: "Slack" }), true)
  assert.equal(NM.mayHaveTarget({ app: "", link: "https://x.example/" }), true)
})

test("snapshot keeps the sender pid only in memory", () => {
  const row = NM.snapshot({ id: 1, appName: "Slack", summary: "hi", body: "", actions: [], hints: { "sender-pid": "4242" } }, "k", 0)
  assert.equal(row.pid, 4242)
  assert.equal(JSON.parse(NM.serialise([row])).rows[0].pid, undefined)
})
