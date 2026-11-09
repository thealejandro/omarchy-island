// Calendars in .ics form: made-up events only.
process.env.TZ = "UTC"

import { test } from "node:test"
import assert from "node:assert/strict"
import { execFileSync } from "node:child_process"
import { mkdtempSync, writeFileSync, mkdirSync, rmSync } from "node:fs"
import { tmpdir } from "node:os"
import path from "node:path"
import { fileURLToPath } from "node:url"
import { load, plain } from "./load.mjs"

const ICS = load("IcsModel.js")
const AG = load("AgendaModel.js")
const root = fileURLToPath(new URL("..", import.meta.url))
const now = new Date(Date.UTC(2026, 10, 9, 10, 0))

const cal = (...events) => ["BEGIN:VCALENDAR", "VERSION:2.0", "PRODID:-//test//EN", ...events, "END:VCALENDAR", ""].join("\r\n")
const vevent = (...lines) => ["BEGIN:VEVENT", ...lines, "END:VEVENT"].join("\r\n")
const parse = (text, opts = {}) => plain(ICS.parse(text, { now, ...opts }))
const pick = list => list.map(e => e.date + " " + (e.time || "--:--") + " " + e.title)

test("single events: all day, UTC, floating; folded and escaped text", () => {
  const out = parse(cal(
    vevent("UID:a", "DTSTART;VALUE=DATE:20261110", "DTEND;VALUE=DATE:20261111", "SUMMARY:Día de prueba"),
    vevent("UID:b", "DTSTART:20261112T143000Z", "SUMMARY:Reunión con\\, coma y una", "  línea plegada"),
    vevent("UID:c", "DTSTART:20261113T080000", "SUMMARY:Flotante"),
  ))
  assert.deepEqual(pick(out), [
    "2026-11-10 --:-- Día de prueba",
    "2026-11-12 14:30 Reunión con, coma y una línea plegada",
    "2026-11-13 08:00 Flotante",
  ])
  assert.equal(out[0].source, "ics")
  assert.equal(out[0].repeat, "none")
  // They fit the agenda as they are.
  for (const e of out) assert.notEqual(AG.clean(e), null)
})

test("multi-day all-day events show on each day; cancelled ones not at all", () => {
  const out = parse(cal(
    vevent("UID:trip", "DTSTART;VALUE=DATE:20261120", "DTEND;VALUE=DATE:20261123", "SUMMARY:Viaje"),
    vevent("UID:gone", "DTSTART:20261121T090000Z", "STATUS:CANCELLED", "SUMMARY:Cancelada"),
  ))
  assert.deepEqual(pick(out), ["2026-11-20 --:-- Viaje", "2026-11-21 --:-- Viaje", "2026-11-22 --:-- Viaje"])
})

test("TZID from the loader's day-by-day offsets, across a DST change", () => {
  // Madrid: +0100 in winter, +0200 in summer (made-up table, a day per entry).
  const start = Date.UTC(2026, 2, 20, 12) / 1000
  const offs = Array.from({ length: 30 }, (_, i) => (i < 9 ? "+0100" : "+0200")).join(",")
  const text = cal(
    vevent("UID:w", "DTSTART;TZID=Europe/Madrid:20260325T090000", "SUMMARY:Antes"),
    vevent("UID:s", "DTSTART;TZID=Europe/Madrid:20260401T090000", "SUMMARY:Después"),
  ) + "X-ISLAND-TZ:Europe/Madrid;" + start + ";" + offs + "\n"
  const out = parse(text, { now: new Date(Date.UTC(2026, 2, 28)) })
  assert.deepEqual(pick(out), ["2026-03-25 08:00 Antes", "2026-04-01 07:00 Después"])
})

test("TZID from the calendar's own VTIMEZONE (Outlook style names)", () => {
  const vtz = [
    "BEGIN:VTIMEZONE", "TZID:Romance Standard Time",
    "BEGIN:STANDARD", "DTSTART:16010101T030000", "TZOFFSETFROM:+0200", "TZOFFSETTO:+0100",
    "RRULE:FREQ=YEARLY;BYDAY=-1SU;BYMONTH=10", "END:STANDARD",
    "BEGIN:DAYLIGHT", "DTSTART:16010101T020000", "TZOFFSETFROM:+0100", "TZOFFSETTO:+0200",
    "RRULE:FREQ=YEARLY;BYDAY=-1SU;BYMONTH=3", "END:DAYLIGHT", "END:VTIMEZONE",
  ].join("\r\n")
  const out = parse(cal(vtz,
    vevent("UID:o1", 'DTSTART;TZID="Romance Standard Time":20261015T100000', "SUMMARY:Verano"),
    vevent("UID:o2", "DTSTART;TZID=Romance Standard Time:20261110T100000", "SUMMARY:Invierno"),
  ))
  assert.deepEqual(pick(out), ["2026-10-15 08:00 Verano", "2026-11-10 09:00 Invierno"])
})

test("an unknown zone is read as local time rather than dropped", () => {
  const out = parse(cal(vevent("UID:u", "DTSTART;TZID=Nowhere/Land:20261110T100000", "SUMMARY:Sin zona")))
  assert.deepEqual(pick(out), ["2026-11-10 10:00 Sin zona"])
})

test("weekly rule with BYDAY, COUNT and an EXDATE", () => {
  const out = parse(cal(vevent("UID:gym", "DTSTART:20261102T070000Z", "SUMMARY:Gimnasio",
    "RRULE:FREQ=WEEKLY;BYDAY=MO,WE;COUNT=5", "EXDATE:20261104T070000Z")))
  assert.deepEqual(pick(out).map(s => s.slice(0, 10)), ["2026-11-02", "2026-11-09", "2026-11-11", "2026-11-16"])
})

test("monthly by nth weekday and by last day; yearly; UNTIL", () => {
  const out = parse(cal(
    vevent("UID:m1", "DTSTART;VALUE=DATE:20260908", "SUMMARY:Segundo martes", "RRULE:FREQ=MONTHLY;BYDAY=2TU;UNTIL=20261231"),
    vevent("UID:m2", "DTSTART;VALUE=DATE:20261031", "SUMMARY:Fin de mes", "RRULE:FREQ=MONTHLY;BYMONTHDAY=-1;COUNT=3"),
    vevent("UID:y1", "DTSTART;VALUE=DATE:20200229", "SUMMARY:Bisiesto", "RRULE:FREQ=YEARLY"),
  ), { from: new Date(Date.UTC(2026, 8, 1)), to: new Date(Date.UTC(2028, 11, 31)) })
  const by = t => pick(out).filter(s => s.endsWith(t)).map(s => s.slice(0, 10))
  assert.deepEqual(by("Segundo martes"), ["2026-09-08", "2026-10-13", "2026-11-10", "2026-12-08"])
  assert.deepEqual(by("Fin de mes"), ["2026-10-31", "2026-11-30", "2026-12-31"])
  assert.deepEqual(by("Bisiesto"), ["2028-02-29"])
})

test("a daily rule from years ago reaches today; INTERVAL is kept", () => {
  const out = parse(cal(vevent("UID:d", "DTSTART:20100101T060000Z", "SUMMARY:Pastilla", "RRULE:FREQ=DAILY;INTERVAL=2")),
    { from: new Date(Date.UTC(2026, 10, 8)), to: new Date(Date.UTC(2026, 10, 14)) })
  const days = pick(out).map(s => s.slice(0, 10))
  assert.ok(days.length >= 3 && days.length <= 4, days.join(","))
  for (let i = 1; i < days.length; i++) assert.equal((Date.parse(days[i]) - Date.parse(days[i - 1])) / 86400000, 2)
})

test("RECURRENCE-ID moves one occurrence; the master skips it", () => {
  const out = parse(cal(
    vevent("UID:std", "DTSTART:20261109T090000Z", "SUMMARY:Daily", "RRULE:FREQ=DAILY;COUNT=3"),
    vevent("UID:std", "RECURRENCE-ID:20261110T090000Z", "DTSTART:20261110T113000Z", "SUMMARY:Daily (movida)"),
  ))
  assert.deepEqual(pick(out), ["2026-11-09 09:00 Daily", "2026-11-10 11:30 Daily (movida)", "2026-11-11 09:00 Daily"])
})

test("rules it can't follow keep their first time; nonsense is ignored", () => {
  const out = parse(cal(
    vevent("UID:h", "DTSTART:20261110T090000Z", "SUMMARY:Cada hora", "RRULE:FREQ=HOURLY"),
    vevent("UID:p", "DTSTART:20261110T090000Z", "SUMMARY:Posición", "RRULE:FREQ=MONTHLY;BYDAY=MO,TU;BYSETPOS=-1"),
    vevent("UID:x", "DTSTART:garbage", "SUMMARY:Rota"),
    vevent("UID:y", "DTSTART:20261110T090000Z"),
    "BEGIN:VEVENT", "SUMMARY:Sin cerrar",
  ))
  assert.deepEqual(pick(out), ["2026-11-10 09:00 Cada hora", "2026-11-10 09:00 Posición"])
  assert.deepEqual(parse(""), [])
  assert.deepEqual(parse(null), [])
  assert.deepEqual(parse("no es un calendario"), [])
})

test("the window and the cap: closest to now are kept, in order", () => {
  const out = parse(cal(vevent("UID:z", "DTSTART:20200101T120000Z", "SUMMARY:Diario", "RRULE:FREQ=DAILY")), { limit: 10 })
  assert.equal(out.length, 10)
  const days = out.map(e => e.date)
  assert.deepEqual(days, [...days].sort())
  assert.ok(days.includes("2026-11-09"))
  const ids = new Set(out.map(e => e.id))
  assert.equal(ids.size, out.length)
  // A year either side of now by default.
  const year = parse(cal(vevent("UID:z", "DTSTART:20200101T120000Z", "SUMMARY:Diario", "RRULE:FREQ=DAILY")), { limit: 5000 })
  assert.ok(year.length > 700 && year.length < 740, String(year.length))
})

test("the same calendar from two sources shows once", () => {
  const one = cal(vevent("UID:dup", "DTSTART:20261110T090000Z", "SUMMARY:Una vez"))
  assert.equal(parse(one + one).length, 1)
})

test("ics-fetch.sh reads folders and files, adds zone offsets, never fails", () => {
  const dir = mkdtempSync(path.join(tmpdir(), "island-ics-"))
  try {
    mkdirSync(path.join(dir, "cal", "trabajo"), { recursive: true })
    writeFileSync(path.join(dir, "cal", "trabajo", "a.ics"),
      cal(vevent("UID:f1", "DTSTART;TZID=Europe/Madrid:20261110T090000", "SUMMARY:De carpeta")))
    writeFileSync(path.join(dir, "suelto.ics"), cal(vevent("UID:f2", "DTSTART:20261111T090000Z", "SUMMARY:Suelto")))
    const out = execFileSync("bash", [path.join(root, "scripts/ics-fetch.sh"),
      path.join(dir, "cal"), " " + path.join(dir, "suelto.ics") + " ", path.join(dir, "no-existe"), "http://example.invalid/x.ics"],
      { encoding: "utf8", env: { ...process.env, XDG_RUNTIME_DIR: dir } })
    assert.match(out, /SUMMARY:De carpeta/)
    assert.match(out, /SUMMARY:Suelto/)
    const events = parse(out)
    assert.deepEqual(pick(events), ["2026-11-10 08:00 De carpeta", "2026-11-11 09:00 Suelto"])
  } finally {
    rmSync(dir, { recursive: true, force: true })
  }
})
