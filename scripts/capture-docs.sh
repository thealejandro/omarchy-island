#!/bin/bash
# Captures the README pictures of the island on workspace 9, with no personal data:
# Do Not Disturb on, a made-up notification history and agenda, a demo track playing
# silently, and no World page (it shows the weather location).
#
# usage: scripts/capture-docs.sh <out-dir>
# Leaves full-screen PNGs and a screen recording in <out-dir>; crop them afterwards.
# Takes about 2 minutes. Don't touch the PC while it runs. On exit (also on error or
# Ctrl+C) it puts back the notification history, theme, background, bar, island
# language, Do Not Disturb, workspace and cursor, and removes the demo agenda.
#
# Needs: grim, gpu-screen-recorder, ydotool (ydotoold running), ffmpeg, jq, mpv with
# the mpris script, and an audio file in $TRACK (any music you may use).
set -u
OUT=${1:?usage: $0 <out-dir>}
TRACK=${TRACK:?set TRACK to an audio file}
mkdir -p "$OUT"
WS=9
MON=$(hyprctl monitors -j | jq -r '.[0].name')
IPC=(qs ipc -p /usr/share/omarchy/shell/shell.qml call)
STATE=$HOME/.local/state/omarchy/island
STORE=$STATE/notifications.json
EVENTS=$STATE/events.json
SHELL_JSON=$HOME/.config/omarchy/shell.json
RAW=$(mktemp -d)

for c in grim gpu-screen-recorder ydotool ffmpeg jq mpv; do
  command -v $c >/dev/null || { echo "missing $c" >&2; exit 1; }
done
[[ -f $TRACK ]] || { echo "missing $TRACK" >&2; exit 1; }
[[ -e $EVENTS ]] && { echo "$EVENTS exists; this script only writes a demo one when there is none" >&2; exit 1; }

# State put back at the end.
ORIG_THEME=$(omarchy theme current 2>/dev/null)
ORIG_BG=$(basename "$(readlink -f "$HOME/.local/state/omarchy/current/background")")
ORIG_BAR_BG=$(jq -r '.bar.background // "opaque"' "$SHELL_JSON")
ORIG_BAR_POS=$(jq -r '.bar.position // "top"' "$SHELL_JSON")
ORIG_WS=$(hyprctl activeworkspace -j | jq -r .id)
ORIG_CUR=$(hyprctl cursorpos)
ORIG_DND=$(omarchy-shell notifications isDnd 2>/dev/null || echo off)
ORIG_LANG=$(jq -r '.. | objects | select(.id? == "thealejandro.island") | .language // empty' "$SHELL_JSON" | head -1)
HAD_STORE=0
[[ -f $STORE ]] && { cp -p "$STORE" "$RAW/notifications.json"; HAD_STORE=1; }
REC_PID=
MPV_PID=
SWAPPED=0

cursor_xy() { hyprctl cursorpos | tr -d ' ' | tr ',' ' '; }
move_to() { # move_to x y ms: eased move, corrected against the real position each step
  local tx=$1 ty=$2 ms=${3:-800} steps i x0 y0 cx cy px py
  read -r x0 y0 < <(cursor_xy)
  steps=$((ms / 16))
  for ((i = 1; i <= steps; i++)); do
    read -r px py < <(awk -v i=$i -v n=$steps -v x0=$x0 -v y0=$y0 -v tx=$tx -v ty=$ty 'BEGIN{
      t=i/n; e=(t<0.5)?4*t*t*t:1-((-2*t+2)^3)/2; printf "%d %d", x0+(tx-x0)*e, y0+(ty-y0)*e }')
    read -r cx cy < <(cursor_xy)
    ydotool mousemove -x $((px - cx)) -y $((py - cy)) >/dev/null 2>&1
    sleep 0.012
  done
  for i in 1 2 3; do
    read -r cx cy < <(cursor_xy)
    ((cx == tx && cy == ty)) && break
    ydotool mousemove -x $((tx - cx)) -y $((ty - cy)) >/dev/null 2>&1
    sleep 0.03
  done
}
click() { ydotool click 0xC0 >/dev/null 2>&1; }
state() { "${IPC[@]}" island state | jq -r ".$1"; }
wait_shell() {
  for _ in $(seq 40); do "${IPC[@]}" island ping >/dev/null 2>&1 && return 0; sleep 0.5; done
  echo "the shell did not come back" >&2; return 1
}
restart_shell() { omarchy restart shell >/dev/null 2>&1; wait_shell; sleep 3; }
shot() { "${IPC[@]}" island open >/dev/null 2>&1; sleep 1.2; grim -o "$MON" "$OUT/$1.png"; echo "shot: $1"; }
page_shot() { "${IPC[@]}" island page "$1" >/dev/null 2>&1; shot "$2"; }

set_lang() {
  omarchy bar set thealejandro.island language "\"$1\"" --json >/dev/null 2>&1
  for _ in $(seq 10); do [[ $(state lang) == "$1" ]] && return 0; sleep 0.3; done
}

restore() {
  [[ -n $REC_PID ]] && kill -INT "$REC_PID" 2>/dev/null && wait "$REC_PID" 2>/dev/null
  [[ -n $MPV_PID ]] && kill "$MPV_PID" 2>/dev/null
  "${IPC[@]}" island close >/dev/null 2>&1
  rm -f "${EVENTS:?}"
  if ((SWAPPED)); then
    # The shell only writes the history a moment after a change; nothing changes now.
    if ((HAD_STORE)); then cp -p "$RAW/notifications.json" "$STORE"; else rm -f "${STORE:?}"; fi
    restart_shell
  fi
  omarchy-shell notifications setDnd "${ORIG_DND:-off}" >/dev/null 2>&1
  if [[ -n $ORIG_LANG ]]; then
    omarchy bar set thealejandro.island language "$(jq -n --arg v "$ORIG_LANG" '$v')" --json >/dev/null 2>&1
  else
    jq 'walk(if type == "object" and .id == "thealejandro.island" then del(.language) else . end)' "$SHELL_JSON" >"$RAW/shell.json" \
      && [[ -s $RAW/shell.json ]] && cat "$RAW/shell.json" >"$SHELL_JSON"
  fi
  omarchy bar position "$ORIG_BAR_POS" >/dev/null 2>&1
  omarchy bar background "$ORIG_BAR_BG" >/dev/null 2>&1
  if [[ -n $ORIG_THEME ]]; then
    omarchy theme set "$ORIG_THEME" >/dev/null 2>&1
    sleep 1.5
    bg=$HOME/.local/state/omarchy/current/theme/backgrounds/$ORIG_BG
    [[ -f $bg ]] && omarchy theme bg set "$bg" >/dev/null 2>&1
  fi
  hyprctl dispatch "hl.dsp.focus({ workspace = \"$ORIG_WS\" })" >/dev/null
  read -r ox oy <<<"$(tr -d ' ' <<<"$ORIG_CUR" | tr ',' ' ')"
  move_to "$ox" "$oy" 300
  rm -rf "${RAW:?}"
  echo "restored: theme $ORIG_THEME, background $ORIG_BG, bar $ORIG_BAR_POS/$ORIG_BAR_BG, workspace $ORIG_WS, dnd ${ORIG_DND:-off}, language ${ORIG_LANG:-auto}"
}
trap restore EXIT
trap 'exit 130' INT TERM

# --- made-up data -----------------------------------------------------------
now=$(date +%s%3N)
min=60000
jq -n --argjson now "$now" --argjson m "$min" '{version: 1, rows: [
  {app: "Messages", appIcon: "internet-chat", summary: "Sam", body: "Lunch at 1? The new place on 5th.", time: ($now - 2*$m), read: false, urgency: 1},
  {app: "Mail", appIcon: "mail-unread", summary: "Weekly report", body: "The draft is ready for review.", time: ($now - 14*$m), read: false, urgency: 1},
  {app: "Calendar", appIcon: "x-office-calendar", summary: "Design review", body: "Starts in 15 minutes", time: ($now - 41*$m), read: true, urgency: 1},
  {app: "Updates", appIcon: "system-software-update", summary: "Updates available", body: "12 packages can be updated.", time: ($now - 95*$m), read: true, urgency: 1}
] | map(. + {iconFile: "", desktopEntry: "", link: ""})}' >"$RAW/demo-store.json"

d() { date -d "$1" +%F; }
jq -n --arg t0 "$(d today)" --arg t1 "$(d tomorrow)" --arg t3 "$(d '+3 days')" --arg t6 "$(d '+6 days')" '{version: 1, events: [
  {id: "demo1", date: $t0, time: "16:30", title: "Design review", repeat: "none", remind: null},
  {id: "demo2", date: $t1, time: "09:00", title: "Team sync", repeat: "weekly", remind: null},
  {id: "demo3", date: $t1, time: "13:00", title: "Lunch with Sam", repeat: "none", remind: null},
  {id: "demo4", date: $t3, time: "", title: "Release day", repeat: "none", remind: null},
  {id: "demo5", date: $t6, time: "19:00", title: "Concert", repeat: "none", remind: null}
]}' >"$RAW/demo-events.json"

ffmpeg -loglevel error -y -i "$TRACK" -t 50 -c copy -metadata title="Limit 70" -metadata artist="Kevin MacLeod" "$RAW/track.m4a"

# --- setup ------------------------------------------------------------------
omarchy-shell notifications setDnd on >/dev/null 2>&1
hyprctl dispatch "hl.dsp.focus({ workspace = \"$WS\" })" >/dev/null
sleep 0.5
move_to 1340 640 300
SWAPPED=1
cp "$RAW/demo-store.json" "$STORE"; chmod 600 "$STORE"
cp "$RAW/demo-events.json" "$EVENTS"
restart_shell
omarchy-shell notifications setDnd on >/dev/null 2>&1
omarchy bar position top >/dev/null 2>&1
omarchy bar background glassy >/dev/null 2>&1
omarchy theme set cupertino >/dev/null 2>&1
sleep 2.5
set_lang en
"${IPC[@]}" island close >/dev/null 2>&1
mpv --no-video --ao=null --no-terminal --loop-file=inf "$RAW/track.m4a" &
MPV_PID=$!
sleep 4

# --- stills (top bar) -------------------------------------------------------
"${IPC[@]}" island peek >/dev/null 2>&1; sleep 1.5
grim -o "$MON" "$OUT/peek.png"; echo "shot: peek"
sleep 3
page_shot home home
page_shot calendar calendar
page_shot notifications notifications
page_shot settings settings
"${IPC[@]}" island close >/dev/null 2>&1; sleep 1

# --- recording: peek, open from the clock, walk the tabs ---------------------
"${IPC[@]}" island page home >/dev/null 2>&1; sleep 0.3; "${IPC[@]}" island close >/dev/null 2>&1; sleep 1
gpu-screen-recorder -w "$MON" -f 60 -fm cfr -k h264 -q very_high -cursor yes -o "$OUT/demo.mp4" >"$RAW/gsr.log" 2>&1 &
REC_PID=$!
sleep 1.5
"${IPC[@]}" island peek >/dev/null 2>&1
sleep 3.2
move_to 960 14 900; sleep 0.35; click; sleep 0.5
[[ $(state pinned) == true ]] || "${IPC[@]}" island open >/dev/null 2>&1
sleep 2.2
for tab in "calendar 1105" "notifications 1163" "settings 1192"; do
  read -r name x <<<"$tab"
  move_to "$x" 50 600; sleep 0.25; click; sleep 0.4
  [[ $(state page) == "$name" ]] || "${IPC[@]}" island page "$name" >/dev/null 2>&1
  sleep 2.2
done
move_to 1340 640 700
"${IPC[@]}" island close >/dev/null 2>&1
sleep 1.5
kill -INT "$REC_PID" 2>/dev/null; wait "$REC_PID" 2>/dev/null; REC_PID=
echo "recorded: demo.mp4"

# --- side and bottom bars ---------------------------------------------------
for pos in left bottom; do
  omarchy bar position $pos >/dev/null 2>&1; sleep 2.5
  page_shot home "bar-$pos"
  "${IPC[@]}" island close >/dev/null 2>&1; sleep 0.8
done
