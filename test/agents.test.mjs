// AI agent status: made-up hook events only.
import { test } from "node:test"
import assert from "node:assert/strict"
import { load, plain } from "./load.mjs"

const AM = load("AgentModel.js")
const IS = load("IslandSettings.js")
const IM = load("IslandModel.js")

const MIN = 60000
const ev = (agent, state, extra = {}) => ({ agent, id: "s1", state, ...extra })

test("states and their aliases", () => {
  for (const s of ["working", "waiting", "done", "error", "idle"]) assert.equal(AM.normalizeState(s), s)
  assert.equal(AM.normalizeState(" Working "), "working")
  assert.equal(AM.normalizeState("busy"), "working")
  assert.equal(AM.normalizeState("permission"), "waiting")
  assert.equal(AM.normalizeState("stop"), "done")
  assert.equal(AM.normalizeState("failed"), "error")
  assert.equal(AM.normalizeState("end"), null)
  assert.equal(AM.normalizeState("clear"), null)
  assert.equal(AM.normalizeState("nonsense"), undefined)
  assert.equal(AM.normalizeState(""), undefined)
  assert.equal(AM.normalizeState("toString"), undefined)
  assert.equal(AM.normalizeState(undefined), undefined)
})

test("text is cleaned: escapes, newlines, controls, direction marks, length", () => {
  assert.equal(AM.cleanText("\x1b[31mred\x1b[0m text", 50), "red text")
  assert.equal(AM.cleanText("\x1b]0;title\x07shown", 50), "shown")
  assert.equal(AM.cleanText("a\nb\r\nc\td\x7fe\x85f", 50), "a b c d e f")
  assert.equal(AM.cleanText("  many     spaces  ", 50), "many spaces")
  assert.equal(AM.cleanText("abc\u202edef\u200b", 50), "abcdef")
  assert.equal(AM.cleanText("x".repeat(500), 160).length, 160)
  assert.equal(AM.cleanText(null, 10), "")
  assert.equal(AM.cleanText("😀😀😀", 3), "😀")
})

test("names, ids, pids and addresses", () => {
  assert.equal(AM.cleanName("Claude Code!"), "claudecode")
  assert.equal(AM.cleanName("a".repeat(100)).length, 32)
  assert.equal(AM.cleanName("../etc"), "..etc")
  assert.equal(AM.cleanId("abc-123_x:y.z/; rm"), "abc-123_x:y.zrm")
  assert.equal(AM.cleanId("i".repeat(200)).length, 64)
  assert.equal(AM.cleanPid("1234"), 1234)
  assert.equal(AM.cleanPid(1234), 1234)
  for (const bad of ["0", "-5", "1.5", "abc", "4194305", "99999999999", "", null, "12 34"]) assert.equal(AM.cleanPid(bad), 0, String(bad))
  assert.equal(AM.cleanAddress("0x5641b30e2f50"), "0x5641b30e2f50")
  assert.equal(AM.cleanAddress("0X5641B30E2F50"), "0X5641B30E2F50")
  for (const bad of ["", "5641b30e", "0x", '0x1"; exec', "0x" + "f".repeat(17), "address:0x1"]) assert.equal(AM.cleanAddress(bad), "", bad)
})

test("projectLabel is the last folder, never the whole path", () => {
  assert.equal(AM.projectLabel("/home/someone/Work/my-project"), "my-project")
  assert.equal(AM.projectLabel("/home/someone/Work/my-project/"), "my-project")
  assert.equal(AM.projectLabel("C:\\Users\\x\\proj"), "proj")
  for (const cwd of ["/", "", null, "..", "/a/.."]) assert.equal(AM.projectLabel(cwd), "", String(cwd))
  const long = AM.projectLabel("/x/" + "p".repeat(300))
  assert.equal(long.length, 48)
  assert.ok(!AM.projectLabel("/home/someone/Work/\x1b[31mevil\nproj").includes("/"))
  assert.equal(AM.projectLabel("/home/someone/Work/\x1b[31mevil\nproj"), "evil proj")
})

test("display names", () => {
  assert.equal(AM.displayName("claude"), "Claude Code")
  assert.equal(AM.displayName("codex"), "Codex")
  assert.equal(AM.displayName("gemini"), "Gemini CLI")
  assert.equal(AM.displayName("mytool"), "Mytool")
  assert.equal(AM.displayName(""), "")
  assert.equal(AM.displayName("constructor"), "Constructor")
})

test("update: upsert, keep what didn't come back, newest first", () => {
  let r = AM.update([], ev("claude", "working", { cwd: "/w/proj", pid: "42", address: "0xabc" }), 1000)
  assert.equal(r.changed, true)
  assert.equal(r.prev, null)
  assert.deepEqual(plain(r.session), { key: "claude:s1", agent: "claude", id: "s1", state: "working", label: "proj",
    message: "", pid: 42, address: "0xabc", since: 1000, updated: 1000, peeked: "" })
  assert.ok(!JSON.stringify(r.list).includes("/w/proj"))
  let list = r.list
  r = AM.update(list, ev("codex", "working"), 2000)
  assert.deepEqual(plain(r.list.map(s => s.key)), ["codex:s1", "claude:s1"])
  assert.equal(list.length, 1, "the old list is left alone")
  list = r.list
  r = AM.update(list, ev("claude", "waiting", { message: "ok?" }), 3000)
  assert.deepEqual(plain(r.list.map(s => s.key)), ["claude:s1", "codex:s1"])
  assert.equal(r.session.label, "proj")
  assert.equal(r.session.pid, 42)
  assert.equal(r.session.address, "0xabc")
  assert.equal(r.session.since, 3000)
  assert.equal(r.prev.state, "working")
  // Same state again: not a change, `since` stays.
  list = r.list
  r = AM.update(list, ev("claude", "waiting", { message: "ok?" }), 4000)
  assert.equal(r.changed, false)
  assert.equal(r.session.since, 3000)
  assert.equal(r.session.updated, 4000)
  // No session id: one default session per agent.
  r = AM.update([], { agent: "aider", state: "working" }, 1)
  assert.equal(r.session.key, "aider:default")
})

test("update: invalid, ended and unknown", () => {
  assert.equal(AM.update([], { agent: "", state: "working" }, 1).invalid, true)
  assert.equal(AM.update([], { agent: "claude", state: "dancing" }, 1).invalid, true)
  assert.equal(AM.update([], null, 1).invalid, true)
  const list = AM.update([], ev("claude", "working"), 1).list
  const r = AM.update(list, ev("claude", "end"), 2)
  assert.equal(r.removed, true)
  assert.deepEqual(plain(r.list), [])
  assert.equal(AM.update(list, ev("claude", "end", { id: "other" }), 2).list.length, 1)
})

test("update: dirty fields are cleaned", () => {
  const r = AM.update([], { agent: "Claude", id: "a b/c", state: "WORKING", cwd: "/x/\x1b[1mp", message: "hi\x1b[0m\nthere", pid: "x", address: "no" }, 1)
  assert.equal(r.session.agent, "claude")
  assert.equal(r.session.id, "abc")
  assert.equal(r.session.message, "hi there")
  assert.equal(r.session.pid, 0)
  assert.equal(r.session.address, "")
})

test("at most 12 sessions: finished ones go before working ones, waiting last", () => {
  let list = []
  let t = 0
  const add = (name, state) => { list = AM.update(list, { agent: "a" + name, state }, ++t).list }
  add("w1", "working")
  add("q1", "waiting")
  for (let i = 0; i < 10; i++) add("d" + i, "done")
  assert.equal(list.length, 12)
  add("n", "working")
  assert.equal(list.length, 12)
  assert.ok(!list.some(s => s.agent === "ad0"), "the oldest done one went")
  assert.ok(list.some(s => s.agent === "aw1") && list.some(s => s.agent === "aq1"))
  // Only working and waiting left: the oldest working one goes.
  list = []
  t = 0
  add("q", "waiting")
  for (let i = 0; i < 11; i++) add("w" + i, "working")
  add("x", "working")
  assert.equal(list.length, 12)
  assert.ok(list.some(s => s.agent === "aq"))
  assert.ok(!list.some(s => s.agent === "aw0"))
  assert.equal(list[0].agent, "ax")
})

test("remove, removeAgent, setWindow, markPeeked", () => {
  let list = AM.update([], ev("claude", "working"), 1).list
  list = AM.update(list, { agent: "claude", id: "s2", state: "done" }, 2).list
  list = AM.update(list, ev("codex", "working"), 3).list
  assert.equal(AM.remove(list, "claude:s1").length, 2)
  assert.deepEqual(plain(AM.removeAgent(list, "claude").map(s => s.key)), ["codex:s1"])
  assert.deepEqual(plain(AM.removeAgent(list, "")), [])
  assert.equal(AM.removeAgent(list, "nobody").length, 3)
  const w = AM.setWindow(list, "codex:s1", "77", "0xf00")
  assert.equal(w.found, true)
  assert.equal(w.list.find(s => s.key === "codex:s1").pid, 77)
  assert.equal(AM.setWindow(list, "nope:x", "77", "0xf00").found, false)
  assert.equal(AM.setWindow(list, "codex:s1", "x", "bad").found, false)
  assert.equal(AM.markPeeked(list, "codex:s1", "done").find(s => s.key === "codex:s1").peeked, "done")
})

test("prune: finished ones go, stuck ones turn idle and then go", () => {
  const stale = 120 * MIN
  let list = AM.update([], ev("a", "done"), 0).list
  list = AM.update(list, ev("b", "error"), 0).list
  list = AM.update(list, ev("c", "working"), 0).list
  list = AM.update(list, ev("d", "waiting"), 0).list
  assert.equal(AM.prune(list, 4 * MIN, stale).length, 4)
  assert.deepEqual(plain(AM.prune(list, 6 * MIN, stale).map(s => s.agent).sort()), ["b", "c", "d"])
  assert.deepEqual(plain(AM.prune(list, 11 * MIN, stale).map(s => s.agent).sort()), ["c", "d"])
  const idle = AM.prune(list, stale + 1, stale)
  assert.deepEqual(plain(idle.map(s => [s.agent, s.state]).sort()), [["c", "idle"], ["d", "idle"]])
  assert.equal(idle[0].since, stale + 1)
  assert.deepEqual(plain(AM.prune(idle, stale + 1 + 61000, stale)), [])
})

test("nextPruneDelay: ms to the next expiry, 0 with nothing to wait for", () => {
  assert.equal(AM.nextPruneDelay([], 0, 120 * MIN), 0)
  let list = AM.update([], ev("a", "working"), 1000).list
  assert.equal(AM.nextPruneDelay(list, 1000, 120 * MIN), 120 * MIN)
  list = AM.update(list, ev("b", "done"), 2000).list
  assert.equal(AM.nextPruneDelay(list, 2000, 120 * MIN), 5 * MIN)
  assert.equal(AM.nextPruneDelay(list, 2000 + 5 * MIN + 10, 120 * MIN), 1, "already due")
  list = AM.update(list, ev("c", "idle"), 3000).list
  assert.equal(AM.nextPruneDelay(list, 3000, 120 * MIN), 60000)
})

test("shouldPeek: waiting, done and error once; working and idle never", () => {
  const s = (state, extra = {}) => ({ key: "a:1", state, peeked: "", ...extra })
  assert.equal(AM.shouldPeek(null, s("waiting")), true)
  assert.equal(AM.shouldPeek(s("working"), s("waiting")), true)
  assert.equal(AM.shouldPeek(s("waiting"), s("waiting")), false)
  assert.equal(AM.shouldPeek(s("waiting"), s("working")), false)
  assert.equal(AM.shouldPeek(s("working"), s("waiting")), true, "again after it worked")
  assert.equal(AM.shouldPeek(null, s("done")), true, "done without a working before")
  assert.equal(AM.shouldPeek(s("working"), s("error")), true)
  assert.equal(AM.shouldPeek(s("done"), s("done")), false)
  assert.equal(AM.shouldPeek(s("done"), s("done", { peeked: "done" })), false)
  assert.equal(AM.shouldPeek(null, s("waiting", { peeked: "waiting" })), false)
  assert.equal(AM.shouldPeek(null, s("working")), false)
  assert.equal(AM.shouldPeek(s("done"), s("idle")), false)
  assert.equal(AM.shouldPeek(null, null), false)
  // Through update and markPeeked, as the service does it.
  let r = AM.update([], ev("claude", "waiting"), 1)
  assert.equal(AM.shouldPeek(r.prev, r.session), true)
  let list = AM.markPeeked(r.list, r.session.key, "waiting")
  r = AM.update(list, ev("claude", "waiting"), 2)
  assert.equal(AM.shouldPeek(r.prev, r.session), false)
})

test("peekNote, summary, listing", () => {
  const s = { agent: "claude", label: "proj", state: "waiting" }
  assert.deepEqual(plain(AM.peekNote(s)), { key: "agentWaiting", arg: "Claude Code", label: "proj", state: "waiting" })
  assert.deepEqual(plain(AM.peekNote({ agent: "codex", label: "", state: "done" })), { key: "agentDone", arg: "Codex", label: "", state: "done" })
  const rows = [{ key: "a", state: "working" }, { key: "b", state: "waiting" }, { key: "c", state: "done" }, { key: "d", state: "waiting" }]
  assert.deepEqual(AM.ordered(rows).map((r) => r.key), ["b", "d", "a", "c"])
  assert.deepEqual(plain(AM.ordered(null)), [])
  assert.equal(AM.peekNote({ agent: "codex", label: "", state: "error" }).key, "agentError")
  assert.equal(AM.peekNote({ agent: "codex", label: "", state: "working" }), null)
  assert.equal(AM.peekNote(null), null)
  const list = [{ state: "working" }, { state: "working" }, { state: "waiting" }, { state: "done" }, { state: "idle" }]
  assert.deepEqual(plain(AM.summary(list)), { working: 2, waiting: 1, done: 1, total: 5 })
  assert.deepEqual(plain(AM.summary(null)), { working: 0, waiting: 0, done: 0, total: 0 })
  const full = AM.update([], ev("claude", "working", { cwd: "/secret/place/proj", message: "m", pid: 5 }), 9).list
  const pub = plain(AM.listing(full, false))
  assert.deepEqual(pub, [{ name: "claude", id: "s1", state: "working", label: "proj", since: 9 }])
  assert.ok(!JSON.stringify(AM.listing(full, true)).includes("secret"))
})

test("focusTarget: the address if it is still there, else the pid, never a guess", () => {
  const NM = load("NotificationModel.js")
  const clients = NM.parseClients(JSON.stringify([
    { address: "0xaaa", class: "Alacritty", title: "claude", pid: 100, focusHistoryID: 1 },
    { address: "0xbbb", class: "kitty", title: "codex", pid: 200, focusHistoryID: 0 }
  ]))
  assert.deepEqual(plain(AM.focusTarget({ address: "0xBBB", pid: 100 }, clients)), { focus: "0xbbb" })
  assert.deepEqual(plain(AM.focusTarget({ address: "0xgone", pid: 100 }, clients)), { focus: "0xaaa" })
  assert.deepEqual(plain(AM.focusTarget({ address: "", pid: 200 }, clients)), { focus: "0xbbb" })
  assert.equal(AM.focusTarget({ address: "", pid: 999 }, clients), null)
  assert.equal(AM.focusTarget({ address: "", pid: 0, agent: "kitty", label: "kitty" }, clients), null)
  assert.equal(AM.focusTarget(null, clients), null)
  assert.equal(AM.focusTarget({ pid: 100 }, null), null)
})

test("parseJson: only the known fields, cleaned, 4 KiB at most", () => {
  const f = plain(AM.parseJson(JSON.stringify({ agent: "Claude", id: "s 1", state: "Waiting", cwd: "/a/b", message: "x\ny",
    pid: "12", address: "0xabc", extra: "dropped" })))
  assert.deepEqual(f, { agent: "claude", id: "s1", state: "waiting", cwd: "/a/b", message: "x y", pid: 12, address: "0xabc" })
  assert.equal(AM.parseJson("not json"), null)
  assert.equal(AM.parseJson("[1,2]"), null)
  assert.equal(AM.parseJson("null"), null)
  assert.equal(AM.parseJson(""), null)
  assert.equal(AM.parseJson(JSON.stringify({ agent: "a", state: "working", message: "x".repeat(5000) })), null)
  assert.notEqual(AM.parseJson(JSON.stringify({ agent: "a", state: "working", message: "x".repeat(3000) })), null)
  // Bytes, not characters.
  assert.equal(AM.parseJson(JSON.stringify({ agent: "a", state: "working", message: "é".repeat(2500) })), null)
  // And it goes through update like a plain event.
  const r = AM.update([], AM.parseJson(JSON.stringify({ agent: "claude", state: "done", cwd: "/p/q" })), 1)
  assert.equal(r.session.label, "q")
})

test("settings: types, defaults and section", () => {
  const want = { agents: ["bool", true], agentPeek: ["bool", true], agentPeekDuration: ["int", 5000],
    agentDot: ["bool", true], agentStale: ["int", 120] }
  for (const [key, [type, def]] of Object.entries(want)) {
    const s = IS.spec(key)
    assert.equal(s.type, type, key)
    assert.equal(s.def, def, key)
    assert.equal(s.section, "system", key)
  }
  const n = IS.normalize({ agentPeekDuration: 99999, agentStale: 1, agents: "false" })
  assert.equal(n.agentPeekDuration, 15000)
  assert.equal(n.agentStale, 10)
  assert.equal(n.agents, false)
})

test("every new key has a translation in English and Spanish", () => {
  const keys = ["agents", "agentPeek", "agentPeekDuration", "agentDot", "agentStale"].map(k => "set_" + k)
    .concat(["hint_agents", "hint_agentDot", "hint_agentStale", "agentWaiting", "agentDone", "agentError", "a11y_agentRow"])
    .concat(AM.STATES.map(s => "agent_" + s))
  for (const k of keys) {
    for (const lang of ["en", "es"]) assert.notEqual(IM.t(lang, k, "x"), k, lang + ": " + k)
  }
  assert.equal(IM.t("en", "agentWaiting", "Codex"), "Codex needs your permission")
  assert.equal(IM.t("es", "agentDone", "Codex"), "Codex ha terminado")
})
