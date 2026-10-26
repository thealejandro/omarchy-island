import QtQuick
import qs.Ui

// Takes no room on the bar. It exists only for the bar API: while the island
// carries the quick toggles, hovering the bar's center (weather, keyboard
// layout, updates) no longer reveals Omarchy's inactive indicators, so the
// bar keeps just the ones that are on. Omarchy's clock and weather panels
// clear the flag when they close, so it's put back whenever that happens.
BarWidget {
  id: root
  moduleName: "thealejandro.island"

  readonly property bool suppress: setting("toggles", "island") === "island"
  readonly property bool suppressed: bar ? bar.centerHoverRevealSuppressed === true : false

  visible: false
  implicitWidth: 0
  implicitHeight: 0

  function apply() {
    if (bar && typeof bar.setCenterHoverRevealSuppressed === "function" && suppressed !== suppress)
      bar.setCenterHoverRevealSuppressed(suppress)
  }

  onSuppressChanged: apply()
  onSuppressedChanged: if (suppress) apply()
  onBarChanged: apply()
  Component.onCompleted: apply()
  Component.onDestruction: if (suppress && bar) bar.setCenterHoverRevealSuppressed(false)
}
