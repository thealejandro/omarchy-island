import { test } from "node:test"
import assert from "node:assert/strict"
import { load, plain } from "./load.mjs"

const Sec = load("NotificationSecurity.js")

test("hostname: DNS names only, no numeric last label", () => {
  assert.equal(Sec.hostname("Example.COM."), "example.com")
  assert.equal(Sec.hostname("localhost"), "")
  assert.equal(Sec.hostname("127.0.0.1"), "")
  assert.equal(Sec.hostname("0x7f.1"), "")
  assert.equal(Sec.hostname("a_b.com"), "")
})

test("safeHttpUrl: http(s) without user info or control characters", () => {
  assert.equal(Sec.safeHttpUrl("https://example.com"), "https://example.com/")
  assert.equal(Sec.safeHttpUrl("HTTPS://Example.com:443/a?b=1"), "https://example.com:443/a?b=1")
  for (const bad of [
    "javascript:alert(1)",
    "file:///etc/passwd",
    "https://user@example.com/",
    "https://example.com/%0a",
    "https://example.com/\"x",
    "https://127.0.0.1/",
    "https://example.com:0/",
    "https://example.com:70000/",
  ]) assert.equal(Sec.safeHttpUrl(bad), "", bad)
})

test("firstLink: first safe link in markup or plain text", () => {
  assert.equal(Sec.firstLink('<a href="javascript:x">a</a> see https://example.org/x.'), "https://example.org/x")
  assert.equal(Sec.firstLink('<a href="https://a.example.com/?a=1&amp;b=2">x</a>'), "https://a.example.com/?a=1&b=2")
  assert.equal(Sec.firstLink("nothing here"), "")
})

test("parseExecArgv: JSON array of strings, fails closed", () => {
  assert.deepEqual(plain(Sec.parseExecArgv('["omarchy-launch-browser","--x"]')), ["omarchy-launch-browser", "--x"])
  assert.deepEqual(plain(Sec.parseExecArgv(["a", "b"])), ["a", "b"])
  for (const bad of ["", "rm -rf /", "[]", '["-x"]', "[1]", '["a\\u0000"]', "{}"])
    assert.equal(Sec.parseExecArgv(bad), null, bad)
})

test("localImagePath: absolute, one spelling, no kernel or runtime dirs", () => {
  assert.equal(Sec.localImagePath("/home/u/a.png"), "/home/u/a.png")
  assert.equal(Sec.localImagePath("file:///home/u/a%20b.png"), "/home/u/a b.png")
  assert.equal(Sec.localImagePath("image://icon//home/u/a.png"), "/home/u/a.png")
  assert.equal(Sec.localImagePath("/run/user/1000/x.png"), "/run/user/1000/x.png")
  for (const bad of ["a.png", "/home/../etc/x", "/home//x", "/proc/self/environ", "/run/secret", "file://host/x", "file:///a?b"])
    assert.equal(Sec.localImagePath(bad), "", bad)
})

test("iconName: icon-theme spelling only", () => {
  assert.equal(Sec.iconName("org.telegram.desktop"), "org.telegram.desktop")
  assert.equal(Sec.iconName("../x"), "")
  assert.equal(Sec.iconName("/usr/x.png"), "")
})
