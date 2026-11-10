import { test } from "node:test"
import assert from "node:assert/strict"
import { load, plain } from "./load.mjs"

const M = load("MediaModel.js")
const F = load("FocusModel.js")
const S = load("IslandSettings.js")
const I = load("IslandModel.js")
const l = F.lengths(S.normalize({}))
const MIN = 60000

test("open width fits scaled and small screens without changing the saved setting", () => {
  assert.equal(M.fitWidth(960, 1920, 1, 18), 960)
  assert.equal(M.fitWidth(960, 1280, 1, 18), 760)
  assert.equal(M.fitWidth(960, 1024, 1, 18), 610)
  assert.equal(M.fitWidth(560, 1920, 1.5, 27), 840)
  assert.equal(M.fitWidth(420, 1920, 1, 18), 420)
  assert.equal(M.fitWidth(560, 400, 1, 18), 364)
  for (let screen = 1; screen <= 3840; screen += 17) {
    for (const scale of [0.75, 1, 1.25, 1.5, 2]) {
      const w = M.fitWidth(960, screen, scale, 18 * scale)
      assert.ok(w >= 0 && w <= Math.max(0, screen - 36 * scale))
      assert.ok(w <= 960 * scale)
    }
  }
})

test("lyrics leave readable room for the player and fall back on a narrow island", () => {
  assert.equal(M.lyricsWidth(330, 16, 180, 160), 0)
  assert.ok(M.lyricsWidth(464, 16, 180, 160) >= 160)
  for (let content = 0; content < 1000; content++) {
    const w = M.lyricsWidth(content, 16, 180, 160)
    assert.ok(w === 0 || (w >= 160 && content - w - 16 >= 180))
  }
})

test("song styles preserve either available part without orphan separators", () => {
  assert.equal(M.songLine("Song", "Artist", "title-artist"), "Song  ·  Artist")
  assert.equal(M.songLine("Song", "Artist", "artist-title"), "Artist  ·  Song")
  assert.equal(M.songLine("Song", "Artist", "title"), "Song")
  assert.equal(M.songLine("Song", "Artist", "artist"), "Artist")
  assert.equal(M.songLine("Song", "", "artist"), "Song")
  assert.equal(M.songLine("", "Artist", "title-artist"), "Artist")
  assert.equal(M.songLine("Song", "", "artist-title"), "Song")
})

test("lyric context is bounded and preserves pauses, prelude and untrusted text as data", () => {
  const lines = Array.from({ length: 600 }, (_, i) => ({ t: i * 1000, text: i === 200 ? "" : `line ${i}` }))
  lines[201].text = '<img src="https://invalid.example/tracker">'
  for (let index = -1; index < lines.length; index++) {
    const rows = M.lyricRows(lines, index)
    assert.ok(rows.length <= 7)
    assert.equal(rows.filter(r => r.distance === 0).length, 1)
    assert.equal(rows.find(r => r.distance === 0).index, index)
  }
  assert.equal(M.lyricRows(lines, -1)[0].text, "")
  assert.equal(M.lyricRows(lines, 200).find(r => r.distance === 0).text, "")
  assert.equal(M.lyricRows(lines, 201).find(r => r.distance === 0).text, lines[201].text)
  assert.deepEqual(plain(M.lyricRows([], -1)), [])
  assert.equal(M.lyricAlpha(0), 1)
  assert.ok(M.lyricAlpha(-1) < M.lyricAlpha(1))
})

test("focus pauses and resumes from a deadline without accumulating missed ticks", () => {
  let s = F.create(l)
  assert.equal(F.active(s), false)
  s = F.toggle(s, 1000)
  assert.equal(F.remaining(s, 1000 + 10 * MIN), 15 * MIN)
  s = F.toggle(s, 1000 + 10 * MIN)
  assert.equal(F.remaining(s, 50 * MIN), 15 * MIN)
  assert.equal(F.active(s), true)
  s = F.toggle(s, 50 * MIN)
  assert.equal(s.deadline, 65 * MIN)
  assert.equal(F.finish(s, 65 * MIN - 1, l), s)
  assert.equal(F.finish(s, 65 * MIN, l).phase, "break")
})

test("long focus durations use the same hour format as desktop", () => {
  assert.equal(I.clock(3599), "59:59")
  assert.equal(I.clock(3600), "1:00:00")
  assert.equal(I.clock(7200), "2:00:00")
  assert.equal(I.clock(3723), "1:02:03")
})

test("focus cycles through short and long breaks, then waits for a click", () => {
  let s = F.create(l), now = 0
  for (let round = 1; round <= l.every; round++) {
    s = F.toggle(s, now)
    now += l.focus * MIN
    s = F.finish(s, now, l)
    assert.equal(s.phase, round === l.every ? "longBreak" : "break")
    assert.equal(s.running, true)
    assert.equal(s.rounds, round)
    now += s.totalMs
    s = F.finish(s, now, l)
    assert.equal(s.phase, "focus")
    assert.equal(s.running, false)
  }
  assert.equal(s.rounds, 0)
})

test("a late wake starts a useful break instead of replaying missed rounds", () => {
  const s = F.toggle(F.create(l), 0)
  const next = F.finish(s, 55 * MIN, l)
  assert.equal(next.phase, "break")
  assert.equal(F.remaining(next, 55 * MIN), 5 * MIN - 1000)
})

test("skipping preserves pause state and new lengths never alter a phase underway", () => {
  const fresh = F.create(l)
  assert.equal(F.advance(fresh, 0, l).running, false)
  const running = F.toggle(fresh, 0)
  assert.equal(F.advance(running, MIN, l).running, true)
  const longer = { ...plain(l), focus: 50 }
  assert.equal(F.resize(fresh, longer).totalMs, 50 * MIN)
  assert.equal(F.resize(running, longer), running)
  const paused = F.toggle(running, MIN)
  assert.equal(F.resize(paused, longer), paused)
})

test("new settings survive saving and have labels in all supported languages", () => {
  const keys = ["openLyrics", "textAlign", "songText", "focusMinutes", "breakMinutes", "longBreakMinutes", "longBreakEvery"]
  for (const key of keys) {
    const spec = S.spec(key)
    assert.ok(spec, key)
    for (const lang of ["en", "es", "pt"]) assert.notEqual(I.t(lang, "set_" + key), "set_" + key)
  }
  assert.equal(S.spec("openWidth").step, 10)
  assert.equal(S.normalize({ openWidth: 960 }).openWidth, 960)
  assert.equal(S.normalize({ openWidth: 420 }).openWidth, 420)
  assert.equal(S.normalize({ openWidth: 725 }).openWidth, 730)
  const saved = S.withValue({ id: "x", custom: 1 }, "focusMinutes", 50, "x")
  assert.equal(S.normalize(saved).focusMinutes, 50)
  assert.equal(saved.custom, 1)
  assert.equal(S.normalize({ defaultPage: "focus" }).defaultPage, "focus")
})
