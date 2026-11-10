import QtQuick
import "FocusModel.js" as FocusModel

// Platform-independent controller. OS notifications belong to the host.
Item {
  id: root
  required property var configuration
  property bool displayActive: false
  readonly property var lengths: FocusModel.lengths(configuration)
  property var session: FocusModel.create(lengths)
  property real now: Date.now()
  readonly property string phase: session.phase
  readonly property int rounds: session.rounds
  readonly property bool running: session.running
  readonly property int remaining: Math.ceil(remainingNow() / 1000)
  readonly property int total: session.totalMs / 1000
  readonly property bool active: FocusModel.active(session)
  signal phaseEnded(string phase)

  // Reading now ensures the display reacts to the boundary timer.
  function remainingNow() { return FocusModel.remaining(session, now) }
  function remainingAt(time) { return FocusModel.remaining(session, time) }
  onLengthsChanged: if (session) session = FocusModel.resize(session, lengths)
  onSessionChanged: {
    now = Date.now()
    alarm.stop()
    if (session.running) {
      alarm.interval = Math.max(1, session.deadline - now)
      alarm.restart()
    }
    refreshDisplay()
  }
  onDisplayActiveChanged: refreshDisplay()

  function toggle() {
    finish()
    session = FocusModel.toggle(session, Date.now())
  }
  function reset() { session = FocusModel.create(lengths) }
  function skip() {
    var finished = phase
    session = FocusModel.advance(session, Date.now(), lengths)
    phaseEnded(finished)
  }
  function finish() {
    var finished = phase
    var next = FocusModel.finish(session, Date.now(), lengths)
    if (next === session) return
    session = next
    phaseEnded(finished)
  }
  function refreshDisplay() {
    display.stop()
    now = Date.now()
    if (!session.running || !displayActive) return
    var left = remainingNow()
    if (left <= 0) { finish(); return }
    display.interval = Math.max(20, left % 1000 || 1000)
    display.restart()
  }
  // One alarm runs while closed. Only a visible display wakes per second.
  Timer { id: alarm; onTriggered: root.finish() }
  Timer { id: display; onTriggered: root.refreshDisplay() }
}
