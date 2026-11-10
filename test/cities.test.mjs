import { test } from "node:test"
import assert from "node:assert/strict"
import { load, plain } from "./load.mjs"

const IC = load("IslandCities.js")

const CITIES = [
  { name: "Tokyo", zone: "Asia/Tokyo" },
  { name: "Madrid", zone: "Europe/Madrid" },
  { name: "New York", zone: "America/New_York" },
]

test("move: up and down, stopping at both ends", () => {
  assert.deepEqual(plain(IC.move(CITIES, 1, -1)), [CITIES[1], CITIES[0], CITIES[2]])
  assert.deepEqual(plain(IC.move(CITIES, 0, 1)), [CITIES[1], CITIES[0], CITIES[2]])
  assert.deepEqual(plain(IC.move(CITIES, 0, -1)), CITIES)
  assert.deepEqual(plain(IC.move(CITIES, 2, 1)), CITIES)
})

test("move and remove: unknown indexes leave the list untouched", () => {
  assert.deepEqual(plain(IC.move(CITIES, -1, 1)), CITIES)
  assert.deepEqual(plain(IC.move(CITIES, 9, 1)), CITIES)
  assert.deepEqual(plain(IC.move(CITIES, NaN, 1)), CITIES)
  assert.deepEqual(plain(IC.remove(CITIES, -1)), CITIES)
  assert.deepEqual(plain(IC.remove(CITIES, 3)), CITIES)
  assert.deepEqual(plain(IC.remove(CITIES, 1)), [CITIES[0], CITIES[2]])
})

test("move and remove do not mutate the input list", () => {
  const copy = plain(CITIES)
  IC.move(CITIES, 0, 1)
  IC.remove(CITIES, 0)
  assert.deepEqual(plain(CITIES), copy)
})

test("add: valid zone appended, empty name derived from the zone", () => {
  const out = IC.add(CITIES, "", "Europe/Paris")
  assert.deepEqual(plain(out), CITIES.concat([{ name: "Paris", zone: "Europe/Paris" }]))
  assert.equal(out.length, 4)
  assert.equal(CITIES.length, 3)
})

test("add: trims, keeps a given name, rejects invalid zones", () => {
  const out = IC.add(CITIES, "  CDMX  ", " America/Mexico_City ")
  assert.deepEqual(plain(out).at(-1), { name: "CDMX", zone: "America/Mexico_City" })
  assert.equal(IC.add(CITIES, "X", ""), null)
  assert.equal(IC.add(CITIES, "X", "not a zone"), null)
  assert.equal(IC.add(CITIES, "X", "../etc"), null)
  assert.equal(IC.add(CITIES, "X", null), null)
})

test("parseZoneLines: keeps valid zones, deduplicates, drops junk", () => {
  const text = "Europe/Madrid\n\n america/new_york \nEurope/Madrid\nnot a zone\n../up\nUTC\n"
  assert.deepEqual(plain(IC.parseZoneLines(text)), ["Europe/Madrid", "america/new_york", "UTC"])
  assert.deepEqual(plain(IC.parseZoneLines("")), [])
  assert.deepEqual(plain(IC.parseZoneLines(null)), [])
})

test("filterZones: case-insensitive substring, slash counts as space", () => {
  const zones = ["Europe/Madrid", "America/Madrid", "Asia/Tokyo"]
  assert.deepEqual(plain(IC.filterZones(zones, "madrid", 8)), ["Europe/Madrid", "America/Madrid"])
  assert.deepEqual(plain(IC.filterZones(zones, "MADRID", 8)), ["Europe/Madrid", "America/Madrid"])
  assert.deepEqual(plain(IC.filterZones(zones, "america madrid", 8)), ["America/Madrid"])
  assert.deepEqual(plain(IC.filterZones(zones, "tok", 8)), ["Asia/Tokyo"])
  assert.deepEqual(plain(IC.filterZones(zones, "zzz", 8)), [])
})

test("filterZones: empty query shows the first zones, limit caps the list", () => {
  const zones = ["A/One", "B/Two", "C/Three", "D/Four"]
  assert.deepEqual(plain(IC.filterZones(zones, "", 2)), ["A/One", "B/Two"])
  assert.deepEqual(plain(IC.filterZones(zones, "", 0)), [])
  assert.deepEqual(plain(IC.filterZones(zones, "  ", 3)), ["A/One", "B/Two", "C/Three"])
  assert.deepEqual(plain(IC.filterZones(null, "", 3)), [])
})

test("isValidZone: same shape the settings parser accepts", () => {
  assert.equal(IC.isValidZone("Europe/Madrid"), true)
  assert.equal(IC.isValidZone("America/Argentina/Buenos_Aires"), true)
  assert.equal(IC.isValidZone("Etc/GMT+1"), true)
  assert.equal(IC.isValidZone("UTC"), true)
  assert.equal(IC.isValidZone("no-slash"), false)
  assert.equal(IC.isValidZone(""), false)
  assert.equal(IC.isValidZone(null), false)
})

test("FALLBACK_ZONES: short and every entry valid", () => {
  assert.ok(IC.FALLBACK_ZONES.length >= 10)
  assert.ok(IC.FALLBACK_ZONES.length <= 40)
  for (const z of IC.FALLBACK_ZONES) assert.ok(IC.isValidZone(z), z)
})
