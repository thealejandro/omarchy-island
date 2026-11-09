// scripts/lyrics-fetch.sh with a fake curl: no request ever leaves the machine.
import { test } from "node:test"
import assert from "node:assert/strict"
import { execFileSync } from "node:child_process"
import { createHash } from "node:crypto"
import { existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, statSync, utimesSync, writeFileSync } from "node:fs"
import { tmpdir } from "node:os"
import path from "node:path"
import { fileURLToPath } from "node:url"
import { load } from "./load.mjs"

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..")
const LM = load("LyricsModel.js")
const script = path.join(root, "scripts/lyrics-fetch.sh")

const TITLE = "Isla de prueba"
const ARTIST = "Artista Ficticio"
const ALBUM = "Demo"
const KEY = LM.cacheKey(ARTIST, TITLE, ALBUM, 180)
const HASH = createHash("sha256").update(KEY).digest("hex")
const RECORD = JSON.stringify({ id: 7, trackName: TITLE, artistName: ARTIST, albumName: ALBUM, duration: 180, instrumental: false,
  plainLyrics: "uno\ndos", syncedLyrics: "[00:05.00]uno\n[00:10.00]dos" })

// A temp home with a curl that answers from files: responses[n] is
// [httpCode, body] for the n-th call, and every call's arguments are logged.
function sandbox(responses) {
  const dir = mkdtempSync(path.join(tmpdir(), "island-lyrics-"))
  mkdirSync(path.join(dir, "bin"))
  mkdirSync(path.join(dir, "cache"))
  responses.forEach(([code, body], i) => {
    writeFileSync(path.join(dir, `code.${i + 1}`), String(code))
    writeFileSync(path.join(dir, `body.${i + 1}`), body)
  })
  writeFileSync(path.join(dir, "bin", "curl"), `#!/usr/bin/env bash
d=${JSON.stringify(dir)}
n=$(( $(cat "$d/count" 2>/dev/null || echo 0) + 1 ))
echo $n >"$d/count"
printf '%s\\n' "$@" >>"$d/args.log"
printf -- '--call--\\n' >>"$d/args.log"
out=""
while [ $# -gt 0 ]; do [ "$1" = -o ] && out=$2; shift; done
[ -f "$d/body.$n" ] && cat "$d/body.$n" >"$out"
cat "$d/code.$n" 2>/dev/null || printf 000
`, { mode: 0o755 })
  return {
    dir,
    cache: path.join(dir, "cache"),
    calls: () => (existsSync(path.join(dir, "count")) ? Number(readFileSync(path.join(dir, "count"), "utf8")) : 0),
    args: () => (existsSync(path.join(dir, "args.log")) ? readFileSync(path.join(dir, "args.log"), "utf8") : ""),
    run(extra = {}, argv = {}) {
      const a = { version: "0.8.0", artist: ARTIST, title: TITLE, album: ALBUM, dur: "180", cartist: "", ctitle: "", key: KEY, ...argv }
      return execFileSync("bash", [script, a.version, a.artist, a.title, a.album, a.dur, a.cartist, a.ctitle, a.key], {
        encoding: "utf8",
        env: { ...process.env, PATH: `${path.join(dir, "bin")}:${process.env.PATH}`, XDG_CACHE_HOME: path.join(dir, "cache"),
          XDG_RUNTIME_DIR: dir, ...extra },
      })
    },
    done: () => rmSync(dir, { recursive: true, force: true }),
  }
}

const lyricsDir = (s) => path.join(s.cache, "omarchy/island/lyrics")

test("a found song is printed and cached under the sha256 of its key", () => {
  const s = sandbox([[200, RECORD]])
  try {
    const out = s.run()
    assert.equal(out.split("\n")[0], "get")
    assert.equal(LM.parseFetch(out, 180).result.kind, "synced")
    assert.equal(readFileSync(path.join(lyricsDir(s), HASH + ".json"), "utf8"), RECORD)
    assert.equal(s.calls(), 1)
  } finally { s.done() }
})

test("arguments: HTTPS only, identified, values are separate arguments", () => {
  const nasty = `Tema & "co" 'x'; $(touch /tmp/no) \`id\` #1`
  const s = sandbox([[200, RECORD]])
  try {
    s.run({}, { title: nasty, artist: "A&B=C", album: "" })
    const args = s.args().split("\n")
    assert.ok(args.includes("--proto"))
    assert.ok(args.includes("=https"))
    assert.ok(args.includes("--proto-redir"))
    assert.ok(args.includes("omarchy-island/0.8.0 (https://github.com/thealejandro/omarchy-island)"))
    assert.ok(args.includes("-G"))
    assert.ok(args.includes("track_name=" + nasty))
    assert.ok(args.includes("artist_name=A&B=C"))
    assert.ok(args.includes("duration=180"))
    assert.ok(!args.some(a => a.startsWith("album_name=")))
    assert.ok(args.includes("https://lrclib.net/api/get"))
    assert.ok(!/http:\/\//.test(s.args()))
    assert.ok(!/@/.test(args.find(a => a.startsWith("omarchy-island/"))))
  } finally { s.done() }
})

test("the album is sent when there is one", () => {
  const s = sandbox([[200, RECORD]])
  try {
    s.run()
    assert.ok(s.args().split("\n").includes("album_name=" + ALBUM))
  } finally { s.done() }
})

test("a seeded cache answers without calling curl, and refreshes its age", () => {
  const s = sandbox([])
  try {
    mkdirSync(lyricsDir(s), { recursive: true })
    const file = path.join(lyricsDir(s), HASH + ".json")
    writeFileSync(file, RECORD)
    const old = new Date(Date.now() - 30 * 86400000)
    utimesSync(file, old, old)
    const out = s.run()
    assert.equal(out.split("\n")[0], "hit")
    assert.equal(LM.parseFetch(out, 180).result.kind, "synced")
    assert.equal(s.calls(), 0)
    assert.ok(Date.now() - statSync(file).mtimeMs < 86400000, "touched")
  } finally { s.done() }
})

test("a cached record older than 90 days is asked for again", () => {
  const s = sandbox([[200, RECORD]])
  try {
    mkdirSync(lyricsDir(s), { recursive: true })
    const file = path.join(lyricsDir(s), HASH + ".json")
    writeFileSync(file, "{}")
    const old = new Date(Date.now() - 91 * 86400000)
    utimesSync(file, old, old)
    assert.equal(s.run().split("\n")[0], "get")
    assert.equal(s.calls(), 1)
  } finally { s.done() }
})

test("not found: remembered for a day, then the second run makes no request", () => {
  const s = sandbox([[404, "{}"], [404, "{}"], [404, "{}"]])
  try {
    assert.equal(s.run().trim(), "notfound")
    assert.equal(s.calls(), 2, "no cleaned names: get then search")
    assert.ok(existsSync(path.join(lyricsDir(s), HASH + ".none")))
    const before = s.calls()
    assert.equal(s.run().trim(), "notfound")
    assert.equal(s.calls(), before)
    // After a day it is tried again.
    const none = path.join(lyricsDir(s), HASH + ".none")
    const old = new Date(Date.now() - 25 * 3600 * 1000)
    utimesSync(none, old, old)
    s.run()
    assert.equal(s.calls(), before + 2)
  } finally { s.done() }
})

test("fallback: cleaned names without the album, then a search", () => {
  const found = JSON.stringify([{ id: 3, duration: 180, syncedLyrics: "[00:01.00]x", plainLyrics: "x", instrumental: false }])
  const s = sandbox([[404, "{}"], [404, "{}"], [200, found]])
  try {
    const out = s.run({}, { title: TITLE + " - Remastered 2011", ctitle: TITLE })
    assert.equal(s.calls(), 3)
    const calls = s.args().split("--call--\n")
    assert.ok(calls[0].includes("track_name=" + TITLE + " - Remastered 2011"))
    assert.ok(calls[0].includes("album_name=" + ALBUM))
    assert.ok(calls[1].includes("track_name=" + TITLE + "\n"))
    assert.ok(!calls[1].includes("album_name="))
    assert.ok(calls[1].includes("https://lrclib.net/api/get"))
    assert.ok(calls[2].includes("https://lrclib.net/api/search"))
    assert.equal(out.split("\n")[0], "search")
    const parsed = LM.parseFetch(out, 180)
    assert.equal(parsed.status, "search")
    assert.equal(parsed.result.kind, "synced")
    // The answer is cached, and a later hit gives the same result.
    const again = s.run({}, { title: TITLE + " - Remastered 2011", ctitle: TITLE })
    assert.equal(again.split("\n")[0], "hit")
    assert.equal(LM.parseFetch(again, 180).result.id, 3)
    assert.equal(s.calls(), 3)
  } finally { s.done() }
})

test("an empty search is a remembered miss", () => {
  const s = sandbox([[404, "{}"], [200, "[]"]])
  try {
    assert.equal(s.run().trim(), "notfound")
    assert.ok(existsSync(path.join(lyricsDir(s), HASH + ".none")))
  } finally { s.done() }
})

test("service errors are reported and nothing is written", () => {
  for (const code of [503, 429, 500, 400]) {
    const s = sandbox([[code, "oops"]])
    try {
      assert.equal(s.run().trim(), "error " + code)
      assert.equal(s.calls(), 1)
      assert.deepEqual(readdirSync(lyricsDir(s)), [])
    } finally { s.done() }
  }
  const s = sandbox([[404, "{}"], [503, "oops"]])
  try {
    assert.equal(s.run().trim(), "error 503")
    assert.deepEqual(readdirSync(lyricsDir(s)), [])
  } finally { s.done() }
})

test("a failing curl and a non-JSON answer are errors", () => {
  const dead = sandbox([])
  try { assert.equal(dead.run().trim(), "error 000") } finally { dead.done() }
  const html = sandbox([[200, "<html>captive portal</html>"]])
  try { assert.equal(html.run().trim(), "error 200") } finally { html.done() }
})

test("no usable cache folder: it still answers, marked nocache", () => {
  const s = sandbox([[200, RECORD]])
  try {
    const file = path.join(s.dir, "not-a-folder")
    writeFileSync(file, "")
    const out = s.run({ XDG_CACHE_HOME: file })
    assert.equal(out.split("\n")[0], "nocache get")
    assert.equal(LM.parseFetch(out, 180).result.kind, "synced")
  } finally { s.done() }
})

test("bad input is refused without a request", () => {
  const s = sandbox([[200, RECORD]])
  try {
    for (const bad of [{ dur: "0" }, { dur: "9999" }, { dur: "abc" }, { dur: "" }, { title: "" }, { artist: "" }])
      assert.equal(s.run({}, bad).trim(), "error 400")
    assert.equal(s.calls(), 0)
  } finally { s.done() }
})

test("pruning: at most 400 songs stay, the oldest go", () => {
  const s = sandbox([[200, RECORD]])
  try {
    const dir = lyricsDir(s)
    mkdirSync(dir, { recursive: true })
    const now = Date.now()
    for (let i = 0; i < 405; i++) {
      const f = path.join(dir, `${String(i).padStart(64, "0")}.json`)
      writeFileSync(f, "{}")
      const t = new Date(now - (405 - i) * 1000 * 60)
      utimesSync(f, t, t)
    }
    s.run()
    const left = readdirSync(dir)
    assert.equal(left.length, 400)
    assert.ok(left.includes(HASH + ".json"))
    assert.ok(!left.includes("0".repeat(64) + ".json"))
    assert.ok(left.includes(String(404).padStart(64, "0") + ".json"))
  } finally { s.done() }
})
