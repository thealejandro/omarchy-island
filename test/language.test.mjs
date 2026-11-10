import { test } from "node:test"
import assert from "node:assert/strict"
import { load, plain, readJson } from "./load.mjs"

const IM = load("IslandModel.js")
const IS = load("IslandSettings.js")

test("Portuguese has exactly the English keys and preserves placeholders and glyphs", () => {
  assert.deepEqual(Object.keys(IM.STRINGS.pt).sort(), Object.keys(IM.STRINGS.en).sort())
  for (const [key, source] of Object.entries(IM.STRINGS.en)) {
    const translated = IM.STRINGS.pt[key]
    assert.ok(translated.trim(), key)
    assert.deepEqual(translated.match(/%\d+/g), source.match(/%\d+/g), key)
    assert.deepEqual(translated.match(/[·←→↑↓…›–°“”]/g), source.match(/[·←→↑↓…›–°“”]/g), key)
  }
  assert.equal(IM.t("pt", "takeBreak", 5), "Faça uma pausa de 5 minutos.")
})

test("Portuguese locale codes resolve to pt", () => {
  for (const code of ["pt", "pt_BR.UTF-8", "pt-BR", "pt_PT.UTF-8"]) {
    assert.equal(IM.language(code), "pt", code)
  }
})

test("language accepts pt in settings and the manifest", () => {
  assert.deepEqual(plain(IS.spec("language").options), ["", "en", "es", "pt"])
  assert.equal(IS.normalize({ language: "pt" }).language, "pt")
  assert.equal(IS.withValue({ id: "x" }, "language", "pt", "x").language, "pt")
  assert.deepEqual(readJson("manifest.json").barWidget.schema.find(s => s.key === "language").options,
    plain(IS.toManifestSchema(null).schema.find(s => s.key === "language").options))
})
