#!/usr/bin/env bash
# Prints the calendars the island should show, for IcsModel.parse():
#   ics-fetch.sh SOURCE...
# Each SOURCE is a folder (searched for *.ics, as vdirsyncer leaves them), an
# .ics file, or an https:// (or webcal://) link such as the private iCal
# address of a Google, Outlook or iCloud calendar. After the calendars come
# "X-ISLAND-TZ:<zone>;<epoch>;<offsets>" lines: each time zone they use, its
# offset from UTC day by day over two years, from the system's tzdata.
#
# Links are fetched over HTTPS only, sizes are capped, and nothing about a
# source (a link holds a secret) is printed to stderr or the log.
set -u

max_total=$((24 * 1024 * 1024))
tmp=$(mktemp "${XDG_RUNTIME_DIR:-/tmp}/island-ics.XXXXXX") || exit 0
trap 'rm -f -- "${tmp:?}"' EXIT

for src in "$@"; do
  src="${src#"${src%%[![:space:]]*}"}"
  src="${src%"${src##*[![:space:]]}"}"
  [ -n "$src" ] || continue
  case "$src" in
    webcal://*) src="https://${src#webcal://}" ;;
  esac
  case "$src" in
    https://*)
      curl -fsL --proto '=https' --proto-redir '=https' --max-time 25 --max-filesize 10000000 \
        -A "omarchy-island" -- "$src" 2>/dev/null | head -c 10000000 >>"$tmp"
      printf '\n' >>"$tmp"
      ;;
    http://*) ;;  # not over plain HTTP: the link is a secret
    *)
      case "$src" in "~"|"~/"*) src="$HOME${src#\~}" ;; esac
      if [ -d "$src" ]; then
        find -L "$src" -maxdepth 4 -type f -name '*.ics' -size -2M -print0 2>/dev/null \
          | xargs -0 -r awk 'FNR==1 && NR>1 { print "" } { print }' 2>/dev/null >>"$tmp"
      elif [ -f "$src" ]; then
        head -c 10000000 -- "$src" >>"$tmp" 2>/dev/null
      fi
      printf '\n' >>"$tmp"
      ;;
  esac
  [ "$(stat -c %s -- "$tmp")" -lt "$max_total" ] || break
done

head -c "$max_total" -- "$tmp"
printf '\n'

# The zones the events name, if the system knows them.
start=$(date -u -d "today 12:00 -400 days" +%s) || exit 0
grep -aoE 'TZID="?[A-Za-z_]+(/[A-Za-z0-9_+-]+)*' -- "$tmp" | sed 's/^TZID="\{0,1\}//' | sort -u | head -n 40 \
  | while IFS= read -r zone; do
      [ -f "/usr/share/zoneinfo/$zone" ] || continue
      case "$zone" in *..*) continue ;; esac
      offsets=$(seq "$start" 86400 $((start + 86400 * 800)) | sed 's/^/@/' \
        | TZ="$zone" date -f - +%z 2>/dev/null | paste -sd, -)
      [ -n "$offsets" ] && printf 'X-ISLAND-TZ:%s;%s;%s\n' "$zone" "$start" "$offsets"
    done
exit 0
