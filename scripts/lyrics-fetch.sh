#!/usr/bin/env bash
# Looks a song up on lrclib.net (or in the local cache) for LyricsModel.parseFetch():
#   lyrics-fetch.sh VERSION ARTIST TITLE ALBUM DURATION CLEAN_ARTIST CLEAN_TITLE KEY
# DURATION is whole seconds (1-3600). CLEAN_ARTIST and CLEAN_TITLE are the
# names without "(Remastered)", "feat. …" and the like, empty when nothing
# changed; they are tried when the exact names are not found. KEY is
# LyricsModel.cacheKey(): its sha256 names the cache file.
#
# Prints a status line, then the JSON when there is one:
#   hit        from the cache
#   get        found by name on lrclib.net
#   search     the answer of a fallback search (a JSON array; the caller picks)
#   notfound   lrclib.net doesn't have it (remembered for a day)
#   error CODE the network or the service failed (nothing is remembered)
# "nocache " is put before a status when the cache folder is not writable.
#
# At most three requests per song, one after another and never retried here.
# Every value travels as a curl argument (curl does the URL encoding), never
# through a shell, and nothing about the song is printed to stderr.
# Cache: ${XDG_CACHE_HOME:-~/.cache}/omarchy/island/lyrics, a month-sized
# LRU of at most 400 songs, found ones kept 90 days, missing ones 24 hours.
set -u

version=${1-}; artist=${2-}; title=${3-}; album=${4-}; dur=${5-}
cartist=${6-}; ctitle=${7-}; key=${8-}

case "$version" in ''|*[!0-9A-Za-z._-]*) version=dev ;; esac
case "$dur" in ''|*[!0-9]*) printf 'error 400\n'; exit 0 ;; esac
if [ "$dur" -lt 1 ] || [ "$dur" -gt 3600 ] || [ -z "$artist" ] || [ -z "$title" ]; then
  printf 'error 400\n'; exit 0
fi

api=https://lrclib.net/api
agent="omarchy-island/$version (https://github.com/thealejandro/island)"
keep=400

tmp=$(mktemp "${XDG_RUNTIME_DIR:-/tmp}/island-lyrics.XXXXXX") || { printf 'error 000\n'; exit 0; }
trap 'rm -f -- "${tmp:?}"' EXIT

dir="${XDG_CACHE_HOME:-$HOME/.cache}/omarchy/island/lyrics"
hash=$(printf '%s' "$key" | sha256sum | cut -c1-64)
usable=0
if [ -n "$key" ] && mkdir -p -- "$dir" 2>/dev/null && [ -d "$dir" ] && [ -w "$dir" ]; then usable=1; fi
prefix=""
[ "$usable" -eq 1 ] || prefix="nocache "

if [ "$usable" -eq 1 ]; then
  if [ -f "$dir/$hash.json" ] && [ -z "$(find "$dir/$hash.json" -mtime +90 2>/dev/null)" ]; then
    touch -- "$dir/$hash.json" 2>/dev/null
    printf 'hit\n'
    cat -- "$dir/$hash.json"
    exit 0
  fi
  if [ -f "$dir/$hash.none" ] && [ -z "$(find "$dir/$hash.none" -mmin +1440 2>/dev/null)" ]; then
    printf 'notfound\n'
    exit 0
  fi
fi

# request ENDPOINT NAME=VALUE...  ->  $code, body in $tmp
request() {
  local endpoint=$1 cap=$2; shift 2
  local args=() kv
  for kv in "$@"; do args+=(--data-urlencode "$kv"); done
  : >"$tmp"
  code=$(curl -sS -G --proto '=https' --proto-redir '=https' --max-time 8 --connect-timeout 4 \
    --max-filesize "$cap" -A "$agent" -o "$tmp" -w '%{http_code}' "${args[@]}" -- "$api/$endpoint" 2>/dev/null) \
    || code=000
  case "$code" in [0-9][0-9][0-9]) ;; *) code=000 ;; esac
}

first_char() { head -c 200 -- "$tmp" | tr -d ' \t\r\n' | head -c 1; }

store() { # store SUFFIX [from-tmp]
  [ "$usable" -eq 1 ] || return 0
  if [ "${2-}" = tmp ]; then
    [ "$(stat -c %s -- "$tmp")" -le 262144 ] || return 0
    cp -- "$tmp" "$dir/.$hash.part" 2>/dev/null && mv -f -- "$dir/.$hash.part" "$dir/$hash.$1" 2>/dev/null
  else
    : >"$dir/$hash.$1" 2>/dev/null
  fi
  prune
}

prune() {
  find "$dir" -maxdepth 1 -type f \( -name '*.json' -o -name '*.none' -o -name '.*.part' \) -mtime +90 -delete 2>/dev/null
  ls -t -- "$dir" 2>/dev/null | tail -n +"$((keep + 1))" | while IFS= read -r f; do rm -f -- "$dir/$f"; done
}

done_with() { # done_with STATUS  (body in $tmp)
  printf '%s%s\n' "$prefix" "$1"
  cat -- "$tmp"
  printf '\n'
  exit 0
}

# 1. By name, as the player gave it.
fields=("track_name=$title" "artist_name=$artist" "duration=$dur")
[ -z "$album" ] || fields+=("album_name=$album")
request get 262144 "${fields[@]}"
if [ "$code" = 200 ] && [ "$(first_char)" = "{" ]; then
  store json tmp
  done_with get
fi
if [ "$code" != 404 ]; then
  printf '%serror %s\n' "$prefix" "$code"
  exit 0
fi

# 2. By the cleaned names, without the album.
[ -n "$ctitle" ] || ctitle=$title
[ -n "$cartist" ] || cartist=$artist
if [ "$ctitle" != "$title" ] || [ "$cartist" != "$artist" ]; then
  request get 262144 "track_name=$ctitle" "artist_name=$cartist" "duration=$dur"
  if [ "$code" = 200 ] && [ "$(first_char)" = "{" ]; then
    store json tmp
    done_with get
  fi
  if [ "$code" != 404 ]; then
    printf '%serror %s\n' "$prefix" "$code"
    exit 0
  fi
fi

# 3. A search; the caller picks the record whose length fits.
request search 1048576 "track_name=$ctitle" "artist_name=$cartist"
if [ "$code" = 200 ] && [ "$(first_char)" = "[" ]; then
  if [ "$(tr -d ' \t\r\n' <"$tmp" | head -c 3)" = "[]" ]; then
    store none
    printf '%snotfound\n' "$prefix"
    exit 0
  fi
  store json tmp
  done_with search
fi
if [ "$code" = 404 ]; then
  store none
  printf '%snotfound\n' "$prefix"
  exit 0
fi
printf '%serror %s\n' "$prefix" "$code"
exit 0
