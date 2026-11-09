import { test } from "node:test"
import assert from "node:assert/strict"
import { readFileSync, readdirSync } from "node:fs"
import { fileURLToPath } from "node:url"
import { load } from "./load.mjs"

const IM = load("IslandModel.js")
const IS = load("IslandSettings.js")
const root = fileURLToPath(new URL("..", import.meta.url))

test("stepChoice: neighbours, stopping at both ends", () => {
  const pages = ["home", "calendar", "world"]
  assert.equal(IM.stepChoice(pages, "home", 1), "calendar")
  assert.equal(IM.stepChoice(pages, "world", 1), "world")
  assert.equal(IM.stepChoice(pages, "home", -1), "home")
  assert.equal(IM.stepChoice(pages, "settings", 1), "home")
  assert.equal(IM.stepChoice([], "home", 1), "home")
})

test("stepChoice compares as JSON, so null and numbers work", () => {
  const week = [null, 1, 6, 7]
  assert.equal(IM.stepChoice(week, null, 1), 1)
  assert.equal(IM.stepChoice(week, 1, -1), null)
  assert.equal(IM.stepChoice(week, 7, 1), 7)
})

test("the keyboard setting is on by default, in the open section", () => {
  const s = IS.spec("keyboard")
  assert.equal(s.type, "bool")
  assert.equal(s.def, true)
  assert.equal(s.section, "open")
})

test("every accessible name the QML asks for exists in both languages", () => {
  const qml = readdirSync(root).filter(f => f.endsWith(".qml"))
  const keys = new Set()
  for (const f of qml) {
    for (const m of readFileSync(root + f, "utf8").matchAll(/"(a11y_[A-Za-z]+)"/g)) keys.add(m[1])
  }
  assert.ok(keys.size >= 15, "found " + keys.size)
  for (const k of [...keys, "set_keyboard", "hint_keyboard"]) {
    for (const lang of ["en", "es"]) assert.notEqual(IM.t(lang, k), k, lang + ": " + k)
  }
})

test("icon-only buttons all have a label", () => {
  for (const f of readdirSync(root).filter(f => f.endsWith(".qml"))) {
    const text = readFileSync(root + f, "utf8")
    for (const m of text.matchAll(/IslandButton \{([^}]*)\}/g)) {
      if (/glyph:/.test(m[1])) assert.match(m[1], /label:/, f + ": " + m[1].trim().slice(0, 60))
    }
  }
})
