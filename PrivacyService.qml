import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Services.Pipewire
import "PrivacyModel.js" as Privacy

// Who is using the microphone or the camera, for the island's dots and
// glance. Loaded only while privacyDots is on.
//
// The microphone comes from PipeWire's own signals: capture streams, minus
// level meters and anything recording a sink's monitor (what's playing,
// not you). Cameras show up there too when an app goes through PipeWire;
// one opened straight through /dev/video* is found with fuser, and only
// when the machine has a camera at all.
Item {
  id: svc

  required property var island

  readonly property var nodes: Pipewire.nodes ? Pipewire.nodes.values : []
  readonly property var groups: Pipewire.linkGroups ? Pipewire.linkGroups.values : []
  // Not sinks, and either a stream, video, or a node Quickshell doesn't type
  // (video capture streams). Bound so their properties come in.
  readonly property var candidates: nodes.filter(function(n) {
    return n && !n.isSink && (n.isStream || n.type === PwNodeType.Untracked || (n.type & PwNodeType.Video))
  })
  PwObjectTracker { objects: svc.candidates }

  property var cameraProcs: []
  property var summary: ({ mic: [], camera: [] })

  function nodeInfo(n) {
    var sources = []
    var held = false
    for (var i = 0; i < groups.length; i++) {
      var g = groups[i]
      if (!g) continue
      if (g.target === n && g.source) sources.push({ isSink: !!g.source.isSink })
      if (g.source === n && (g.state === PwLinkState.Active || g.state === PwLinkState.Paused)) held = true
    }
    return { name: String(n.name || ""), isStream: !!n.isStream, isSink: !!n.isSink,
      props: n.properties || {}, linkSources: sources, inUse: held }
  }

  function recompute() {
    var infos = []
    for (var i = 0; i < candidates.length; i++) {
      var n = candidates[i]
      // A muted stream isn't hearing anything (as Omarchy's own mic widget).
      if (n.audio && n.audio.muted) continue
      infos.push(nodeInfo(n))
    }
    var next = Privacy.summarize(infos, cameraProcs)
    var was = summary
    summary = next
    island.micApps = next.mic
    island.cameraApps = next.camera
    if (ready && island.cfg.privacyPeek && Privacy.started(was, next)) island.peek("privacy")
  }

  Timer { id: settle; interval: 150; onTriggered: svc.recompute() }
  onCandidatesChanged: settle.restart()
  onGroupsChanged: settle.restart()
  onCameraProcsChanged: settle.restart()
  // Properties arrive a moment after a node is bound.
  Instantiator {
    model: svc.candidates
    delegate: Connections {
      required property var modelData
      target: modelData
      ignoreUnknownSignals: true
      function onPropertiesChanged() { settle.restart() }
      function onReadyChanged() { settle.restart() }
    }
  }

  // Whatever was already recording when the shell started isn't news.
  property bool ready: false
  Timer { interval: 3000; running: true; onTriggered: svc.ready = true }

  // ---------------------------------------------------------------- camera

  property bool hasCamera: false
  Process {
    running: true
    command: ["sh", "-c", "ls /dev/video* >/dev/null 2>&1 && echo yes"]
    stdout: StdioCollector { onStreamFinished: svc.hasCamera = text.trim() === "yes" }
  }
  Process {
    id: fuserRun
    command: ["sh", "-c", 'for p in $(fuser /dev/video* 2>/dev/null); do p=${p%%[!0-9]*}; '
      + '[ -n "$p" ] && printf "%s\\t%s\\n" "$p" "$(cat /proc/$p/comm 2>/dev/null)"; done']
    stdout: StdioCollector {
      onStreamFinished: {
        var next = Privacy.parseFuser(text)
        if (JSON.stringify(next) !== JSON.stringify(svc.cameraProcs)) svc.cameraProcs = next
      }
    }
  }
  // Often while the island is in view, seldom otherwise; never without a camera.
  Timer {
    interval: svc.island.isOpen || svc.island.peeking ? 5000 : 15000
    repeat: true
    running: svc.hasCamera
    triggeredOnStart: true
    onTriggered: if (!fuserRun.running) fuserRun.running = true
  }

  Component.onDestruction: {
    island.micApps = []
    island.cameraApps = []
  }
}
