import { test } from "node:test"
import assert from "node:assert/strict"
import { load, plain, readJson } from "./load.mjs"

const AG = load("AgendaModel.js")
const IS = load("IslandSettings.js")

test("normalizeTime: short forms, all day, rejects", () => {
  assert.equal(AG.normalizeTime("9:05"), "09:05")
  assert.equal(AG.normalizeTime("0905"), "09:05")
  assert.equal(AG.normalizeTime(""), "")
  assert.equal(AG.normalizeTime("25:00"), null)
  assert.equal(AG.normalizeTime("abc"), null)
})

test("clean: bad dates and empty titles dropped, repeat defaulted", () => {
  assert.equal(AG.clean({ title: "x", date: "2026-02-30" }), null)
  assert.equal(AG.clean({ title: " ", date: "2026-02-03" }), null)
  const ev = AG.clean({ title: " Gym ", date: "2026-02-03", time: "7:00", repeat: "daily", remind: "-1" })
  assert.deepEqual(plain(ev), { id: "", date: "2026-02-03", time: "07:00", title: "Gym", repeat: "none", remind: null, source: "local" })
})

test("occursOn: weekly, monthly and yearly repeats from the start date", () => {
  const ev = { date: "2026-01-05", repeat: "weekly" } // a Monday
  assert.equal(AG.occursOn(ev, new Date(2026, 0, 12)), true)
  assert.equal(AG.occursOn(ev, new Date(2026, 0, 13)), false)
  assert.equal(AG.occursOn(ev, new Date(2025, 11, 29)), false)
  assert.equal(AG.occursOn({ date: "2026-01-31", repeat: "monthly" }, new Date(2026, 2, 31)), true)
  assert.equal(AG.occursOn({ date: "2024-03-10", repeat: "yearly" }, new Date(2026, 2, 10)), true)
})

test("eventsOn: all day first, then by time", () => {
  const list = AG.eventsOn([
    { title: "b", date: "2026-03-01", time: "10:00", repeat: "none" },
    { title: "a", date: "2026-03-01", time: "", repeat: "none" },
    { title: "c", date: "2026-03-01", time: "08:30", repeat: "none" },
  ], "2026-03-01")
  assert.deepEqual(plain(list.map(e => e.title)), ["a", "c", "b"])
  assert.equal(list[0].on, "2026-03-01")
})

test("nextEvents: skips today's timed events over an hour ago, honours the limit", () => {
  const now = new Date(2026, 2, 1, 12, 0)
  const events = [
    { title: "past", date: "2026-03-01", time: "10:00", repeat: "none" },
    { title: "recent", date: "2026-03-01", time: "11:30", repeat: "none" },
    { title: "tomorrow", date: "2026-03-02", time: "", repeat: "none" },
    { title: "later", date: "2026-03-03", time: "", repeat: "none" },
  ]
  assert.deepEqual(plain(AG.nextEvents(events, now, 7, 2).map(e => e.title)), ["recent", "tomorrow"])
})

test("settings: normalize gives every key a value", () => {
  const out = IS.normalize(null)
  for (const s of IS.SCHEMA) assert.ok(s.key in out, s.key)
})

test("settings: withValue keeps the id and drops defaults", () => {
  const s = IS.SCHEMA.find(x => x.type === "bool")
  const set = IS.withValue({ id: "x", other: 1 }, s.key, !s.def, "x")
  assert.equal(set.id, "x")
  assert.equal(set.other, 1)
  assert.equal(set[s.key], !s.def)
  assert.equal(s.key in IS.withValue(set, s.key, s.def, "x"), false)
})

test("manifest.json matches the settings schema", () => {
  const manifest = readJson("manifest.json").barWidget
  const gen = plain(IS.toManifestSchema(null))
  assert.deepEqual(manifest.defaults, gen.defaults)
  // Same entries; the order in manifest.json is free.
  const strip = list => Object.fromEntries(list.map(({ label, ...rest }) => [rest.key, rest]))
  assert.deepEqual(strip(manifest.schema), strip(gen.schema))
})
