import QtQuick
import QtTest
import "../.." as Island

TestCase {
  name: "FocusController"
  when: windowShown

  Component {
    id: controller
    Island.FocusTimer {
      configuration: ({focusMinutes:25, breakMinutes:5, longBreakMinutes:15, longBreakEvery:4})
    }
  }
  SignalSpy { id: ended; signalName: "phaseEnded" }

  function makeTimer() {
    var timer = createTemporaryObject(controller, this)
    verify(timer !== null)
    ended.target = timer
    ended.clear()
    return timer
  }

  function test_alarm_completes_while_hidden() {
    var timer = makeTimer()
    compare(timer.remaining, 1500)
    compare(timer.displayActive, false)
    timer.session = {phase:"focus", rounds:0, running:true,
      deadline:Date.now()+80, leftMs:80, totalMs:80}
    tryCompare(timer, "phase", "break", 1000)
    compare(timer.running, true)
    compare(timer.rounds, 1)
    compare(timer.total, 300)
    compare(ended.count, 1)
    compare(ended.signalArguments[0][0], "focus")
    timer.reset()
  }

  function test_paused_timer_does_not_expire() {
    var timer = makeTimer()
    timer.session = {phase:"focus", rounds:0, running:false,
      deadline:0, leftMs:100, totalMs:100}
    wait(150)
    compare(timer.phase, "focus")
    compare(ended.count, 0)
    timer.toggle()
    tryCompare(timer, "phase", "break", 1000)
    compare(ended.count, 1)
    timer.reset()
  }

  function test_lengths_persist_without_altering_a_running_phase() {
    var timer = makeTimer()
    timer.toggle()
    timer.configuration = {focusMinutes:50, breakMinutes:8, longBreakMinutes:20, longBreakEvery:2}
    compare(timer.total, 1500)
    timer.skip()
    compare(timer.phase, "break")
    compare(timer.total, 480)
    timer.skip()
    compare(timer.running, false)
    compare(timer.total, 3000)
    timer.toggle()
    timer.skip()
    compare(timer.phase, "longBreak")
    compare(timer.total, 1200)
    timer.skip()
    compare(timer.rounds, 0)
    compare(timer.running, false)
  }

  function test_open_display_catches_up_from_deadline() {
    var timer = makeTimer()
    timer.toggle()
    timer.now = Date.now() - 5000
    verify(timer.remaining > 1500)
    timer.displayActive = true
    verify(timer.remaining <= 1500)
    timer.toggle()
    var left = timer.remaining
    wait(60)
    compare(timer.remaining, left)
    timer.reset()
  }
}
