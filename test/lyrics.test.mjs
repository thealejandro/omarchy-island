// Synced lyrics: made-up songs and records only.
import { test } from "node:test"
import assert from "node:assert/strict"
import { load, plain } from "./load.mjs"

const LM = load("LyricsModel.js")
const IS = load("IslandSettings.js")
const IM = load("IslandModel.js")

const times = (r) => plain(r.lines.map(l => l.t))
const texts = (r) => plain(r.lines.map(l => l.text))

test("parseTime: the usual forms, and rejects", () => {
  assert.equal(LM.parseTime("00:17.12"), 17120)
  assert.equal(LM.parseTime("01:02.345"), 62345)
  assert.equal(LM.parseTime("1:02"), 62000)
  assert.equal(LM.parseTime("00:05.5"), 5500)
  assert.equal(LM.parseTime("1:02:03.50"), 3723500)
  assert.equal(LM.parseTime("00:61.00"), -1)
  assert.equal(LM.parseTime("ar:Alguien"), -1)
  assert.equal(LM.parseTime(null), -1)
})

test("parseLrc: several stamps on a line give several entries, in order", () => {
  const r = LM.parseLrc("[00:12.00][00:45.50]Estribillo\n[00:20.00]Verso")
  assert.deepEqual(times(r), [12000, 20000, 45500])
  assert.deepEqual(texts(r), ["Estribillo", "Verso", "Estribillo"])
  assert.equal(r.count, 3)
})

test("parseLrc: two and three decimals, none at all", () => {
  const r = LM.parseLrc("[00:01.5]a\n[00:02.25]b\n[00:03.125]c\n[00:04]d")
  assert.deepEqual(times(r), [1500, 2250, 3125, 4000])
})

test("parseLrc: offset moves the lines (+ earlier), metadata is ignored", () => {
  const r = LM.parseLrc("[ar:Artista Ficticio]\n[ti:Isla de prueba]\n[al:Demo]\n[by:nadie]\n[length:03:00]\n[offset:+300]\n[00:10.00]uno\n[00:20.00]dos")
  assert.equal(r.offset, 300)
  assert.deepEqual(times(r), [9700, 19700])
  const neg = LM.parseLrc("[offset:-500]\n[00:10.00]uno")
  assert.deepEqual(times(neg), [10500])
  // Never before zero.
  assert.deepEqual(times(LM.parseLrc("[offset:5000]\n[00:01.00]x")), [0])
})

test("parseLrc: empty stamped lines stay, word stamps and controls go", () => {
  const r = LM.parseLrc("[00:01.00]hola <00:01.20>mundo <00:01.50>fin\n[00:05.00]\n[00:06.00]\u001b[31mrojo​\u0007")
  assert.deepEqual(texts(r), ["hola mundo fin", "", "rojo"])
})

test("parseLrc: CRLF, BOM, lines without stamps, equal stamps keep their order", () => {
  const r = LM.parseLrc("﻿[00:02.00]b\r\nsin marca\r\n[00:01.00]a\r\n[00:02.00]c")
  assert.deepEqual(texts(r), ["a", "b", "c"])
})

test("parseLrc: caps on lines, line length and text size", () => {
  let big = ""
  for (let i = 0; i < 2000; i++) big += `[${String(Math.floor(i / 60)).padStart(2, "0")}:${String(i % 60).padStart(2, "0")}.00]x\n`
  assert.equal(LM.parseLrc(big).lines.length, LM.MAX_LINES)
  assert.equal(LM.parseLrc("[00:01.00]" + "a".repeat(5000)).lines[0].text.length, LM.MAX_LINE)
  const huge = "[00:01.00]hola\n" + "x".repeat(LM.MAX_TEXT * 2) + "\n[00:09.00]perdida"
  assert.deepEqual(texts(LM.parseLrc(huge)), ["hola"])
})

test("parseLrc: not a string gives nothing", () => {
  for (const v of [null, undefined, 5, {}, ""]) assert.deepEqual(plain(LM.parseLrc(v)), { lines: [], offset: 0, count: 0 })
})

const song = LM.parseLrc("[00:05.00]uno\n[00:10.00]\n[00:15.00]tres\n[00:20.00]cuatro").lines

test("lineAt: before, exact, between and after", () => {
  assert.equal(LM.lineAt(song, 0), -1)
  assert.equal(LM.lineAt(song, 4999), -1)
  assert.equal(LM.lineAt(song, 5000), 0)
  assert.equal(LM.lineAt(song, 12000), 1)
  assert.equal(LM.lineAt(song, 99999), 3)
  assert.equal(LM.lineAt([], 5), -1)
})

test("lineAt agrees with a linear search", () => {
  let seed = 7
  const rnd = () => (seed = (seed * 1103515245 + 12345) % 2147483648) / 2147483648
  const lines = []
  let t = 0
  for (let i = 0; i < 120; i++) { t += Math.floor(rnd() * 6000); lines.push({ t, text: "l" + i }) }
  for (let n = 0; n < 200; n++) {
    const pos = Math.floor(rnd() * (t + 5000))
    let want = -1
    for (let i = 0; i < lines.length; i++) if (lines[i].t <= pos) want = i
    assert.equal(LM.lineAt(lines, pos), want)
  }
})

test("window: next skips gaps, nextAt is the next stamp, -1 at the end", () => {
  assert.deepEqual(plain(LM.window(song, 2000)), { index: -1, current: "", next: "uno", nextAt: 5000 })
  assert.deepEqual(plain(LM.window(song, 6000)), { index: 0, current: "uno", next: "tres", nextAt: 10000 })
  assert.deepEqual(plain(LM.window(song, 11000)), { index: 1, current: "", next: "tres", nextAt: 15000 })
  assert.deepEqual(plain(LM.window(song, 30000)), { index: 3, current: "cuatro", next: "", nextAt: -1 })
})

test("normalizeTitle: streaming suffixes go", () => {
  assert.equal(LM.normalizeTitle("Canción Inventada - Remastered 2011"), "Canción Inventada")
  assert.equal(LM.normalizeTitle("Canción Inventada - 2009 Remaster"), "Canción Inventada")
  assert.equal(LM.normalizeTitle("Canción Inventada (Remastered)"), "Canción Inventada")
  assert.equal(LM.normalizeTitle("Canción Inventada (feat. Otra Persona)"), "Canción Inventada")
  assert.equal(LM.normalizeTitle("Canción Inventada [ft. Otra Persona]"), "Canción Inventada")
  assert.equal(LM.normalizeTitle("Canción Inventada ft. Otra Persona"), "Canción Inventada")
  assert.equal(LM.normalizeTitle("Canción Inventada (Official Video)"), "Canción Inventada")
  assert.equal(LM.normalizeTitle("Canción Inventada (Official Music Video) [HD]"), "Canción Inventada")
  assert.equal(LM.normalizeTitle("Canción Inventada (Lyrics)"), "Canción Inventada")
  assert.equal(LM.normalizeTitle("Canción Inventada (Visualizer)"), "Canción Inventada")
  assert.equal(LM.normalizeTitle("Canción Inventada (Bonus Track)"), "Canción Inventada")
  assert.equal(LM.normalizeTitle("Canción Inventada (Deluxe Edition)"), "Canción Inventada")
})

test("normalizeTitle: what changes the song stays", () => {
  for (const t of ["Canción Inventada (Live)", "Canción Inventada (Acoustic)", "Canción Inventada (Radio Edit)", "Canción Inventada - Live in Madrid", "Plain title"])
    assert.equal(LM.normalizeTitle(t), t)
  assert.equal(LM.normalizeTitle("(Remastered)"), "(Remastered)")
})

test("normalizeArtist: the first of the list", () => {
  assert.equal(LM.normalizeArtist("Artista Uno, Artista Dos"), "Artista Uno")
  assert.equal(LM.normalizeArtist("Artista Uno & Artista Dos"), "Artista Uno")
  assert.equal(LM.normalizeArtist("Artista Uno feat. Artista Dos"), "Artista Uno")
  assert.equal(LM.normalizeArtist("Artista Uno x Artista Dos"), "Artista Uno")
  assert.equal(LM.normalizeArtist("Artista Uno; Artista Dos"), "Artista Uno")
  assert.equal(LM.normalizeArtist("Artista Solo"), "Artista Solo")
})

test("cacheKey: stable, ignores case and spacing, rounds the duration", () => {
  const k = LM.cacheKey("Artista Ficticio", "Isla de prueba", "Demo", 180)
  assert.equal(k, "artista ficticio\nisla de prueba\ndemo\n180")
  assert.equal(LM.cacheKey("  ARTISTA   Ficticio ", "Isla  de PRUEBA", "demo", 179.6), k)
  assert.notEqual(LM.cacheKey("Artista Ficticio", "Isla de prueba", "Demo", 181), k)
  assert.equal(LM.cacheKey("a", "b", "", 0), "a\nb\n\n0")
})

const rec = (extra) => ({ id: 42, trackName: "Isla de prueba", duration: 180, instrumental: false, plainLyrics: null, syncedLyrics: null, ...extra })

test("validate: synced, plain, instrumental and nothing", () => {
  const s = LM.validate(rec({ syncedLyrics: "[00:05.00]uno\n[00:10.00]dos", plainLyrics: "uno\ndos" }), 180)
  assert.equal(s.kind, "synced")
  assert.equal(s.id, 42)
  assert.equal(s.lines.length, 2)
  assert.equal(LM.validate(rec({ plainLyrics: "uno\ndos" }), 180).kind, "plain")
  assert.equal(LM.validate(rec({ instrumental: true }), 180).kind, "instrumental")
  assert.equal(LM.validate(rec({}), 180).kind, "none")
  assert.equal(LM.validate({}, 180).kind, "none")
  // Synced text with no stamps is not synced.
  assert.equal(LM.validate(rec({ syncedLyrics: "sin marcas" }), 180).kind, "none")
})

test("validate: accepts JSON text, rejects broken, huge and wrong shapes", () => {
  assert.equal(LM.validate(JSON.stringify(rec({ syncedLyrics: "[00:01.00]x" })), 180).kind, "synced")
  for (const bad of ["{roto", "", "[]", "null", "5", null, undefined, "x".repeat(300000)])
    assert.equal(LM.validate(bad, 180).kind, "none")
  assert.equal(LM.validate(rec({ syncedLyrics: "[00:01.00]" + "x".repeat(LM.MAX_TEXT) }), 180).kind, "none")
})

test("validate: a record whose duration is far from the track's is not used", () => {
  const r = rec({ syncedLyrics: "[00:01.00]x", duration: 200 })
  assert.equal(LM.validate(r, 180).kind, "none")
  assert.equal(LM.validate(r, 198).kind, "synced")
})

test("pickSearch: closest duration with synced lyrics", () => {
  const list = [
    rec({ id: 1, duration: 150, syncedLyrics: "[00:01.00]a" }),
    rec({ id: 2, duration: 182, syncedLyrics: "[00:01.00]b" }),
    rec({ id: 3, duration: 180, plainLyrics: "solo texto" }),
    rec({ id: 4, duration: 179, syncedLyrics: "[00:01.00]c" }),
    null, 7,
  ]
  assert.equal(LM.pickSearch(list, 180).id, 4)
  assert.equal(LM.pickSearch([], 180), null)
  assert.equal(LM.pickSearch(null, 180), null)
  assert.equal(LM.pickSearch([rec({ duration: 100, syncedLyrics: "[00:01.00]a" })], 180), null)
})

test("parseFetch: every status the script prints", () => {
  const synced = JSON.stringify(rec({ syncedLyrics: "[00:01.00]x" }))
  for (const status of ["hit", "get"]) {
    const r = LM.parseFetch(`${status}\n${synced}`, 180)
    assert.equal(r.status, status)
    assert.equal(r.result.kind, "synced")
    assert.equal(r.retryMs, 0)
  }
  const viaCache = LM.parseFetch(`nocache get\n${synced}`, 180)
  assert.equal(viaCache.status, "get")
  assert.equal(viaCache.result.kind, "synced")
  const search = LM.parseFetch(`search\n${JSON.stringify([rec({ id: 9, syncedLyrics: "[00:01.00]x" })])}`, 180)
  assert.equal(search.status, "search")
  assert.equal(search.result.id, 9)
  const empty = LM.parseFetch("search\n[]", 180)
  assert.equal(empty.status, "notfound")
  assert.equal(empty.retryMs, 24 * 3600 * 1000)
  assert.equal(LM.parseFetch("notfound\n", 180).retryMs, 24 * 3600 * 1000)
  for (const e of ["error 429", "error 503", "error 000", "", "basura", null]) {
    const r = LM.parseFetch(e, 180)
    assert.equal(r.status, "error")
    assert.equal(r.result, null)
    assert.equal(r.retryMs, 10 * 60 * 1000)
  }
})

test("parseFetch: a cached search answer (a list) is picked from again", () => {
  const list = JSON.stringify([rec({ id: 5, syncedLyrics: "[00:01.00]x" })])
  assert.equal(LM.parseFetch(`hit\n${list}`, 180).result.id, 5)
  assert.equal(LM.parseFetch("hit\n[]", 180).status, "notfound")
})

test("settings and strings: off by default, in General, in both languages", () => {
  for (const key of ["lyrics", "lyricsPeek"]) {
    const s = IS.spec(key)
    assert.equal(s.type, "bool")
    assert.equal(s.def, false)
    assert.equal(s.section, "general")
    for (const lang of ["en", "es"]) {
      assert.notEqual(IM.t(lang, "set_" + key), "set_" + key)
      assert.notEqual(IM.t(lang, "hint_" + key), "hint_" + key)
    }
  }
  for (const lang of ["en", "es"]) {
    assert.notEqual(IM.t(lang, "a11y_lyric", "x"), "a11y_lyric")
    assert.match(IM.t(lang, "a11y_lyric", "x"), /x/)
    assert.notEqual(IM.t(lang, "lyricsBy"), "lyricsBy")
  }
})
