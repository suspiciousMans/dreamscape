#!/usr/bin/env bash
# Shoots every dream type with the autopilot under a virtual display and
# fails if any frame is missing or blank. Usage: screenshots.sh OUT_DIR
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
themes="Lobby LiminalOffice VoidPlatforms Garden NightmareFactory CursedForest DrownedLibrary SkyStairs MirrorHall Nightmare"
fail=0
for t in $themes; do
  shot="$out/$t.png"
  rm -f "$shot"
  extra=()
  if [ "$t" = Nightmare ]; then extra=(DREAMSCAPE_NIGHTMARE=1); t_theme=MirrorHall; else t_theme="$t"; fi
  env "${extra[@]}" LIBGL_ALWAYS_SOFTWARE=1 SDL_AUDIODRIVER=dummy DREAMSCAPE_AUTOPILOT=1 \
    DREAMSCAPE_SEED=3 DREAMSCAPE_THEME="$t_theme" DREAMSCAPE_BOOKLET="$out/booklet.ron" \
    DREAMSCAPE_SHOT="$shot" DREAMSCAPE_SHOT_AT="${SHOT_AT:-5}" \
    timeout 120 ./target/debug/dreamscape >"$out/$t.log" 2>&1 || true
  if [ ! -s "$shot" ]; then
    echo "FAIL $t: no screenshot (see $out/$t.log)"; fail=1; continue
  fi
  # A blank or single-colour frame means nothing rendered.
  sd=$(convert "$shot" -colorspace gray -format '%[fx:standard_deviation]' info: 2>/dev/null || echo 1)
  if awk "BEGIN{exit !($sd < 0.02)}"; then
    echo "FAIL $t: frame is blank (stddev $sd)"; fail=1
  else
    echo "ok   $t (stddev $sd)"
  fi
done
exit $fail
