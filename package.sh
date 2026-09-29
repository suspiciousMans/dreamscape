#!/usr/bin/env bash
# Builds a game in release mode and assembles a distributable folder under
# dist/<game>/ (Linux and Steam Deck; the Windows twin is package.ps1).
#
#   ./package.sh dreamscape            # dist/dreamscape/
#   ./package.sh dreamscape --tar      # also dist/dreamscape-linux.tar.gz
#   FEATURES=steam ./package.sh dreamscape
set -euo pipefail

game="${1:?usage: package.sh <game> [--tar]}"
tar_it="${2:-}"
root="$(cd "$(dirname "$0")" && pwd)"
game_dir="$root/games/$game"
[ -d "$game_dir" ] || { echo "No such game: games/$game" >&2; exit 1; }

echo "Building '$game' in release mode..."
args=(build --release -p "$game")
[ -n "${FEATURES:-}" ] && args+=(--features "$FEATURES")
(cd "$root" && cargo "${args[@]}")

exe="$root/target/release/$game"
[ -x "$exe" ] || { echo "Expected build output not found: $exe" >&2; exit 1; }

dist="$root/dist/$game"
rm -rf "$dist"
mkdir -p "$dist"
cp "$exe" "$dist/"
strip "$dist/$game" 2>/dev/null || true

# Same folders as package.ps1; never "saves" (runtime state, made on launch).
for folder in assets profiles levels rigs classes scripts sfx music; do
  [ -d "$game_dir/$folder" ] && cp -r "$game_dir/$folder" "$dist/$folder"
done

# Steam's runtime passes through; a steam_appid.txt only matters for dev runs.
if [ -f "$game_dir/steam_appid.txt" ] && [[ "${FEATURES:-}" == *steam* ]]; then
  cp "$game_dir/steam_appid.txt" "$dist/"
fi

echo "Packaged to $dist"
(cd "$dist" && find . -maxdepth 2 | sort | sed 's|^\./|  |')

if [ "$tar_it" = "--tar" ]; then
  tarball="$root/dist/$game-linux.tar.gz"
  tar -czf "$tarball" -C "$root/dist" "$game"
  echo "Tarred to $tarball"
fi
