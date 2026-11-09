#!/usr/bin/env bash
# Tells the island what an AI coding agent is doing, from the agent's hooks.
#
#   agent-hook.sh <agent> [state] [json]
#
# <agent>  claude, codex, gemini, aider… (a name; shown capitalised)
# [state]  working | waiting | done | error | idle | end. Left out, it is
#          worked out from the hook's JSON (hook_event_name) read on stdin.
# [json]   a payload given as the last argument instead of stdin (Codex's
#          `notify` does that).
#
# A hook must never get in the agent's way: whatever goes wrong (no jq, no
# shell running, a payload that isn't JSON) this exits 0 without a word.
# The working directory is sent to the island, which keeps only the name of
# its last folder.
set -euo pipefail
trap 'exit 0' ERR

agent=${1:-}
[[ -n $agent ]] || exit 0
shift
state=""
payload=""
if (( $# > 0 )) && [[ ${*: -1} == \{* ]]; then
  payload=${*: -1}
  set -- "${@:1:$#-1}"
fi
(( $# > 0 )) && state=$1

command -v jq >/dev/null 2>&1 || exit 0
command -v omarchy-shell >/dev/null 2>&1 || exit 0

if [[ -z $payload && ! -t 0 ]]; then
  # Hooks close stdin; one left open must not hang the agent.
  payload=$(timeout 2 head -c 65536 || true)
fi
[[ -n $payload ]] || payload='{}'

# The window the agent runs in: the nearest ancestor of this process that
# Hyprland has a window for, so a click in the island can bring it up.
address=""
pid=""
if [[ -n ${HYPRLAND_INSTANCE_SIGNATURE:-} ]] && command -v hyprctl >/dev/null 2>&1; then
  declare -A windows=()
  clients=$(timeout 2 hyprctl clients -j 2>/dev/null || true)
  while read -r p a; do
    [[ -n ${p:-} && -n ${a:-} ]] && windows[$p]=$a
  done < <(jq -r '.[]? | "\(.pid) \(.address)"' <<<"$clients" 2>/dev/null || true)
  p=$PPID
  for _ in $(seq 1 40); do
    (( p > 1 )) || break
    if [[ -n ${windows[$p]:-} ]]; then
      pid=$p
      address=${windows[$p]}
      break
    fi
    p=$(sed -E 's/^.*\) [A-Za-z] ([0-9]+) .*$/\1/' "/proc/$p/stat" 2>/dev/null || true)
    [[ $p =~ ^[0-9]+$ ]] || break
  done
fi

message=$(jq -c -n --arg agent "$agent" --arg state "$state" --arg pid "$pid" --arg address "$address" \
  --argjson input "$(jq -c 'if type == "object" then . else {} end' <<<"$payload" 2>/dev/null || echo '{}')" '
  def event: $input.hook_event_name // "";
  def kind: $input.notification_type // "";
  ( if $state != "" then $state
    elif ($input | has("agent_id")) then ""
    elif event == "UserPromptSubmit" then "working"
    elif event == "PermissionRequest" then "waiting"
    elif event == "Notification" then
      (if (kind | test("^(permission_prompt|elicitation_dialog|agent_needs_input)$")) then "waiting"
       elif kind == "idle_prompt" then "idle" else "" end)
    elif event == "Stop" then "done"
    elif event == "StopFailure" then "error"
    elif event == "SessionEnd" then "end"
    else "" end ) as $s
  | if $s == "" then empty else
      { agent: $agent, state: $s,
        id: ($input.session_id // $input["thread-id"] // $input.thread_id // ""),
        cwd: ($input.cwd // ""),
        message: (if event == "Notification" then (($input.message // "") | tostring | .[0:160]) else "" end) }
      + (if $pid != "" then { pid: $pid } else {} end)
      + (if $address != "" then { address: $address } else {} end)
    end' 2>/dev/null || true)

[[ -n $message ]] || exit 0
timeout 3 omarchy-shell -q island agentJson "$message" >/dev/null 2>&1 || true
exit 0
