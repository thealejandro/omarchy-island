// Closed-island notch and hour thread: pure logic, setting and strings.
import { test } from "node:test"
import assert from "node:assert/strict"
import { load, plain } from "./load.mjs"

const II = load("IslandIdle.js")
const IS = load("IslandSettings.js")
const IM = load("IslandModel.js")

test("look: off draws nothing, whatever the state", () => {
  for (const busy of [false, true]) assert.equal(II.look("off", "closed", false, busy), "none")
})

test("look: notch shows only with the island closed and the bar visible", () => {
  assert.equal(II.look("notch", "closed", false, false), "notch")
  assert.equal(II.look("notch", "closed", false, true), "notch")
  assert.equal(II.look("notch", "open", false, false), "none")
  assert.equal(II.look("notch", "peek", false, false), "none")
  assert.equal(II.look("notch", "closed", true, false), "none")
})

test("look: hour yields to busy and is a plain notch then", () => {
  assert.equal(II.look("hour", "closed", false, false), "hour")
  assert.equal(II.look("hour", "closed", false, true), "notch")
  assert.equal(II.look("hour", "open", false, false), "none")
  assert.equal(II.look("hour", "closed", true, false), "none")
})

test("look: day yields to busy and is a plain notch then", () => {
  assert.equal(II.look("day", "closed", false, false), "day")
  assert.equal(II.look("day", "closed", false, true), "notch")
  assert.equal(II.look("day", "peek", false, false), "none")
  assert.equal(II.look("day", "closed", true, false), "none")
})

test("look: battery needs a battery and yields to busy", () => {
  assert.equal(II.look("battery", "closed", false, false, true), "battery")
  assert.equal(II.look("battery", "closed", false, false, false), "notch")
  assert.equal(II.look("battery", "closed", false, false), "notch")
  assert.equal(II.look("battery", "closed", false, true, true), "notch")
  assert.equal(II.look("battery", "open", false, false, true), "none")
  assert.equal(II.look("battery", "closed", true, false, true), "none")
})

test("look: an unknown setting draws nothing", () => {
  assert.equal(II.look("eyes", "closed", false, false), "none")
  assert.equal(II.look(undefined, "closed", false, false), "none")
})

test("busy: each flag alone counts; empty doesn't; music doesn't", () => {
  for (const k of ["unread", "mic", "camera", "agent", "timer"]) assert.equal(II.busy({ [k]: true }), true, k)
  assert.equal(II.busy({}), false)
  assert.equal(II.busy(null), false)
  assert.equal(II.busy({ unread: false, mic: false, camera: false, agent: false, timer: false }), false)
  assert.equal(II.busy({ playing: true }), false)
})

test("fractionOfMinute grows with the minute and stays inside 0..1", () => {
  assert.ok(II.fractionOfMinute(0) > 0 && II.fractionOfMinute(0) < 0.01)
  assert.ok(II.fractionOfMinute(59) > 0.99 && II.fractionOfMinute(59) < 1)
  for (let m = 1; m < 60; m++) assert.ok(II.fractionOfMinute(m) > II.fractionOfMinute(m - 1))
  assert.equal(II.fractionOfMinute(-1), 0)
  assert.equal(II.fractionOfMinute(NaN), 0)
  assert.equal(II.fractionOfMinute(75), II.fractionOfMinute(59))
})

test("threadLength: floor minPx, ceiling length, rounded", () => {
  assert.equal(II.threadLength(0, 50, 2), 2)
  assert.equal(II.threadLength(1, 50, 2), 50)
  assert.equal(II.threadLength(0.5, 50, 2), 25)
  assert.equal(II.threadLength(0.333, 50, 2), 17)
  assert.equal(II.threadLength(-3, 50, 2), 2)
  assert.equal(II.threadLength(9, 50, 2), 50)
  assert.equal(II.threadLength(0.5, 1, 2), 1)
  assert.equal(II.threadLength(NaN, 50, 2), 2)
})

test("msUntilNextMinute is always 1..60000", () => {
  assert.equal(II.msUntilNextMinute(0), 60000)
  assert.equal(II.msUntilNextMinute(120000), 60000)
  assert.equal(II.msUntilNextMinute(59999), 1)
  assert.equal(II.msUntilNextMinute(60001), 59999)
  for (const t of [1, 17, 30000, 59999, 1.7e12, 1.7e12 + 1234]) {
    const ms = II.msUntilNextMinute(t)
    assert.ok(ms >= 1 && ms <= 60000, String(t))
  }
})

test("idleNotch setting: enum off|notch|icon|hour|day|battery, default notch, look section", () => {
  const spec = plain(IS.spec("idleNotch"))
  assert.equal(spec.type, "enum")
  assert.deepEqual(spec.options, ["off", "notch", "icon", "hour", "day", "battery"])
  assert.equal(spec.def, "notch")
  assert.equal(spec.section, "look")
  assert.equal(IS.normalize({ idleNotch: "eyes" }).idleNotch, "notch")
  assert.equal(IS.normalize({ idleNotch: "hour" }).idleNotch, "hour")
  assert.equal(IS.normalize({}).idleNotch, "notch")
})

test("idleNotch: setting it back to the default clears the stored value", () => {
  const stored = plain(IS.withValue({ idleNotch: "hour" }, "idleNotch", "notch"))
  assert.equal("idleNotch" in stored, false)
  assert.equal(plain(IS.withValue({}, "idleNotch", "hour")).idleNotch, "hour")
})

test("idleNotch strings exist in English and Spanish", () => {
  for (const lang of ["en", "es"]) {
    const t = (k) => IM.t(lang, k)
    for (const k of ["set_idleNotch", "hint_idleNotch", "opt_notch", "opt_hour", "opt_day", "opt_battery", "opt_off"]) {
      assert.notEqual(t(k), k, `${lang} ${k}`)
    }
  }
})

test("fractionOfDay: quarter-hour steps, middle of the step", () => {
  assert.equal(II.fractionOfDay(0), 0.5 / 96)
  assert.equal(II.fractionOfDay(14), 0.5 / 96)
  assert.equal(II.fractionOfDay(15), 1.5 / 96)
  assert.equal(II.fractionOfDay(720), 48.5 / 96)
  assert.equal(II.fractionOfDay(1439), 95.5 / 96)
  assert.ok(II.fractionOfDay(1439) < 1)
  assert.equal(II.fractionOfDay(5000), II.fractionOfDay(1439))
  assert.equal(II.fractionOfDay(-1), 0)
  assert.equal(II.fractionOfDay(NaN), 0)
  for (let q = 1; q < 96; q++) assert.ok(II.fractionOfDay(q * 15) > II.fractionOfDay(q * 15 - 1))
})

test("fractionOfBattery: level / 100, clamped, no battery is 0", () => {
  assert.equal(II.fractionOfBattery(0), 0)
  assert.equal(II.fractionOfBattery(50), 0.5)
  assert.equal(II.fractionOfBattery(100), 1)
  assert.equal(II.fractionOfBattery(101), 1)
  assert.equal(II.fractionOfBattery(-1), 0)
  assert.equal(II.fractionOfBattery(NaN), 0)
  assert.equal(II.fractionOfBattery(undefined), 0)
})

test("batteryLow: 20% or less, discharging, with a battery", () => {
  assert.equal(II.batteryLow(20, false), true)
  assert.equal(II.batteryLow(0, false), true)
  assert.equal(II.batteryLow(21, false), false)
  assert.equal(II.batteryLow(10, true), false)
  assert.equal(II.batteryLow(-1, false), false)
})

test("msUntilNextQuarter is always 1..900000 and lands on :00 :15 :30 :45", () => {
  assert.equal(II.msUntilNextQuarter(0, 0, 0), 900000)
  assert.equal(II.msUntilNextQuarter(15, 0, 0), 900000)
  assert.equal(II.msUntilNextQuarter(14, 59, 999), 1)
  assert.equal(II.msUntilNextQuarter(1, 0, 0), 840000)
  assert.equal(II.msUntilNextQuarter(1439, 59, 999), 1)
  assert.equal(II.msUntilNextQuarter(1440 - 15, 0, 0), 900000)
  assert.equal(II.msUntilNextQuarter(7, 30, 500), 900000 - (7 * 60000 + 30500))
  for (let m = 0; m < 1440; m += 7) for (const [s, ms] of [[0, 0], [30, 250], [59, 999]]) {
    const w = II.msUntilNextQuarter(m, s, ms)
    assert.ok(w >= 1 && w <= 900000, `${m}:${s}.${ms}`)
    assert.equal((m * 60000 + s * 1000 + ms + w) % 900000, 0)
  }
})
