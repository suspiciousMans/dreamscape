#!/usr/bin/env bash
# Builds the browser version into games/dreamscape/web/dist/:
# index.html + dreamscape.{js,wasm,data}. Needs the Emscripten SDK on PATH
# (source emsdk_env.sh) and `rustup target add wasm32-unknown-emscripten`.
# If emcc can't download its SDL2 port, point it at a local SDL checkout:
#   EMCC_LOCAL_PORTS=sdl2=/path/to/SDL (tag release-2.32.10).
set -euo pipefail
cd "$(dirname "$0")/../../.."
cargo build -p dreamscape --target wasm32-unknown-emscripten --release
out=games/dreamscape/web/dist
mkdir -p "$out"
cp games/dreamscape/web/index.html "$out/"
cp target/wasm32-unknown-emscripten/release/deps/dreamscape.{js,wasm,data} "$out/"
echo "built $out (serve it over http, e.g. python3 -m http.server -d $out)"
