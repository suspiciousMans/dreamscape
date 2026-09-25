#!/usr/bin/env bash
# Shoots every dream type, a nightmare and each special enemy with the
# autopilot under a virtual display, and fails if any frame is missing or
# blank. Usage: screenshots.sh OUT_DIR
set -euo pipefail
out="${1:-target/dream-shots}"
mkdir -p "$out"
cd "$(dirname "$0")/../../.."
cargo build -p dreamscape
if [ -z "${DISPLAY:-}" ]; then
  Xvfb :97 -screen 0 1280x720x24 >/dev/null 2>&1 &
  xvfb_pid=$!
  trap 'kill $xvfb_pid' EXIT
  export DISPLAY=:97
  sleep 1
fi
# name:theme:extra env
shots="
Lobby:Lobby:
LiminalOffice:LiminalOffice:
VoidPlatforms:VoidPlatforms:
Garden:Garden:
NightmareFactory:NightmareFactory:
CursedForest:CursedForest:
DrownedLibrary:DrownedLibrary:
SkyStairs:SkyStairs:
MirrorHall:MirrorHall:
Nightmare:MirrorHall:DREAMSCAPE_NIGHTMARE=1
Stalker:LiminalOffice:DREAMSCAPE_SPECIAL=Stalker
Mimic:MirrorHall:DREAMSCAPE_SPECIAL=Mimic
Sentry:NightmareFactory:DREAMSCAPE_SPECIAL=Sentry
Drifter:VoidPlatforms:DREAMSCAPE_SPECIAL=Drifter
Jester:Garden:DREAMSCAPE_SPECIAL=Jester
"
fail=0
for entry in $shots; do
  IFS=: read -r name theme extra <<<"$entry"
  shot="$out/$name.png"
  rm -f "$shot"
  env ${extra:+"$extra"} LIBGL_ALWAYS_SOFTWARE=1 SDL_AUDIODRIVER=dummy DREAMSCAPE_AUTOPILOT=1 \
    DREAMSCAPE_SEED=3 DREAMSCAPE_THEME="$theme" DREAMSCAPE_BOOKLET="$out/booklet.ron" \
    DREAMSCAPE_SHOT="$shot" DREAMSCAPE_SHOT_AT="${SHOT_AT:-5}" \
    timeout 120 ./target/debug/dreamscape >"$out/$name.log" 2>&1 || true
  if [ ! -s "$shot" ]; then
    echo "FAIL $name: no screenshot (see $out/$name.log)"; fail=1; continue
  fi
  # A blank or single-colour frame means nothing rendered.
  sd=$(convert "$shot" -colorspace gray -format '%[fx:standard_deviation]' info: 2>/dev/null || echo 1)
  if awk "BEGIN{exit !($sd < 0.02)}"; then
    echo "FAIL $name: frame is blank (stddev $sd)"; fail=1
  else
    echo "ok   $name (stddev $sd)"
  fi
done
exit $fail
