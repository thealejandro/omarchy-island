import { test } from "node:test"
import assert from "node:assert/strict"
import { load, plain } from "./load.mjs"

const NM = load("NotificationModel.js")

const rows = (prefix, n) => Array.from({ length: n }, (_, i) => ({ key: prefix + i }))
const groups = [
  { key: "app:slack", rows: rows("s", 2) },
  { key: "web:github.com", rows: rows("g", 4) },
]

test("isFolded: big groups start folded, a hand choice wins", () => {
  assert.equal(NM.isFolded({}, groups[0]), false)
  assert.equal(NM.isFolded({}, groups[1]), true)
  assert.equal(NM.isFolded({ "web:github.com": false }, groups[1]), false)
  assert.equal(NM.isFolded({ "app:slack": true }, groups[0]), true)
})

test("keyboardOrder follows the page: groups (folded show one), then earlier", () => {
  const earlier = rows("h", 5)
  assert.deepEqual(plain(NM.keyboardOrder(groups, {}, earlier, 3)), ["s0", "s1", "g0", "h0", "h1", "h2"])
  assert.deepEqual(plain(NM.keyboardOrder(groups, { "web:github.com": false }, [], 30)),
    ["s0", "s1", "g0", "g1", "g2", "g3"])
  assert.deepEqual(plain(NM.keyboardOrder([], {}, [])), [])
})

test("keyCommand", () => {
  assert.deepEqual(plain(NM.keyCommand("Down")), { command: "next" })
  assert.deepEqual(plain(NM.keyCommand("k")), { command: "prev" })
  assert.deepEqual(plain(NM.keyCommand("enter")), { command: "activate" })
  assert.deepEqual(plain(NM.keyCommand("delete")), { command: "dismiss" })
  assert.deepEqual(plain(NM.keyCommand("escape")), { command: "exit" })
  assert.deepEqual(plain(NM.keyCommand("3")), { command: "action", n: 3 })
  assert.equal(NM.keyCommand("0"), null)
  assert.equal(NM.keyCommand("x"), null)
  assert.equal(NM.keyCommand("constructor"), null)
  assert.equal(NM.keyCommand(undefined), null)
})

test("stepKey stops at both ends and starts at the top", () => {
  const order = ["a", "b", "c"]
  assert.equal(NM.stepKey(order, "a", 1), "b")
  assert.equal(NM.stepKey(order, "c", 1), "c")
  assert.equal(NM.stepKey(order, "a", -1), "a")
  assert.equal(NM.stepKey(order, "gone", 1), "a")
  assert.equal(NM.stepKey([], "a", 1), "")
})

test("keepSelection: same card, else the one below, else above", () => {
  assert.equal(NM.keepSelection(["a", "b", "c"], ["c", "a", "b"], "b"), "b")
  assert.equal(NM.keepSelection(["a", "b", "c"], ["a", "c"], "b"), "c")
  assert.equal(NM.keepSelection(["a", "b", "c"], ["a", "b"], "c"), "b")
  assert.equal(NM.keepSelection(["a"], ["x"], "a"), "x")
  assert.equal(NM.keepSelection([], ["x", "y"], ""), "x")
  assert.equal(NM.keepSelection(["a"], [], "a"), "")
})
