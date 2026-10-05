# Third-Party Notices

Dreamscape ships with and links against third-party components. This file
records their licenses. The Dreamscape game itself is proprietary — see
[`LICENSE`](LICENSE). The `engine/` crate is MIT — see [`engine/LICENSE`](engine/LICENSE).

---

## Bundled assets

### VT323 — SIL Open Font License 1.1

- **Used by:** `games/dreamscape/assets/fonts/VT323-Regular.ttf`
- **Author:** The VT323 Project Authors (Peter Hull)
- **License:** SIL Open Font License, Version 1.1
- **Full text:** `games/dreamscape/assets/fonts/OFL.txt` (shipped alongside the font)

The OFL permits commercial use, bundling in an application, and redistribution,
provided the font is not sold on its own and this notice and the OFL text
travel with it. Both conditions are met: the font ships inside the game, and
`OFL.txt` is packaged alongside it by `package.sh` / `package.ps1`.

VT323 declares no Reserved Font Name, so no rename obligation applies — not for
the unmodified font, and not for a modified one either. If you ever fork the
font, re-check the copyright header in `OFL.txt`: that clause is a property of
the font version, not a permanent fact.

### All other assets — original

Every other asset in this repository is original work: textures are generated
procedurally in code (plasma, swirl, eyes, kaleidoscope patterns), all sound
effects are synthesized at runtime with no audio files, and models are plain
`.obj` meshes authored here. There are no other bundled third-party assets.

---

## Rust dependencies

There are 310 crates in the resolved graph. **296 are permissively licensed**
(MIT, Apache-2.0, Zlib, BSD, ISC, CC0, Unicode-3.0, or a choice among them) and
pose no obligation beyond carrying their notices.

**Three require attention.** They do not oblige you to open-source Dreamscape —
all three are file-level or optional — but you should know they are there:

### symphonia 0.5.5 and its 11 sub-crates — MPL-2.0

Pulled in by `rodio` (enabled with `features = ["symphonia-all"]`) to decode
MP3, FLAC, AAC, Vorbis, ADPCM, WAV and MP4 audio. Twelve crates total:
`symphonia`, `-core`, `-metadata`, `-utils-xiph`, `-codec-pcm`,
`-codec-aac`, `-codec-adpcm`, `-codec-vorbis`, `-bundle-flac`, `-bundle-mp3`,
`-format-isomp4`, `-format-riff`.

MPL-2.0 is **file-level copyleft**, not project-level. It says: if you *modify*
one of these crates' files, you must publish those modified files under MPL-2.0.
It says nothing about the code that calls it. Linking an unmodified symphonia
into a proprietary game is compatible with the license — the MPL files stay
under MPL, your game stays yours.

So the obligation is narrow and easy to keep: **do not patch symphonia
locally.** If a bug ever forces a fix, keep the patch as a separate patch file
and publish it alongside the fork rather than editing it in place.

### epaint_default_fonts 0.29.1 — (MIT OR Apache-2.0) AND OFL-1.1 AND LicenseRef-UFL-1.0

The default font data compiled into `egui`, which the engine's editor GUI uses.
Dual-licensed: the crate code is MIT/Apache-2.0, the embedded font files are
OFL-1.1 (with a Unicode License Exception). The exception removes the
reserved-font-name and bitmap-embedding restrictions, so shipping it inside a
closed-source binary is fine. No attribution action needed beyond noting the
dependency.

Note this is a second OFL font in the graph, separate from the VT323 you ship
yourself. Both are fine; they are simply independent.

### r-efi 5.3.0 — MIT OR Apache-2.0 OR LGPL-2.1-or-later

A UEFI target-description library, reachable only through
`winit`/`raw-window-handle` on a Windows target. The `OR` means you take MIT and
the LGPL option never applies. Listed here so the LGPL string in the graph
doesn't read as a problem later.

### Direct dependencies

Versions and license expressions for the crates you name in `Cargo.toml`:

| Crate | Version | License |
|---|---|---|
| anyhow | 1.0.103 | MIT OR Apache-2.0 |
| bincode | 1.3.3 | MIT |
| bytemuck | 1.25.1 | Zlib OR Apache-2.0 OR MIT |
| egui | 0.29.1 | MIT OR Apache-2.0 |
| egui_glow | 0.29.1 | MIT OR Apache-2.0 |
| env_logger | 0.11.11 | MIT OR Apache-2.0 |
| glam | 0.29.3 | MIT OR Apache-2.0 |
| glow | 0.14.2 | MIT OR Apache-2.0 OR Zlib |
| gltf | 1.4.1 | MIT OR Apache-2.0 |
| hecs | 0.10.5 | MIT OR Apache-2.0 |
| image | 0.25.10 | MIT OR Apache-2.0 |
| log | 0.4.33 | MIT OR Apache-2.0 |
| notify | 6.1.1 | CC0-1.0 |
| rand | 0.8.8 | MIT OR Apache-2.0 |
| rfd | 0.15.4 | MIT |
| rodio | 0.19.0 | MIT OR Apache-2.0 |
| ron | 0.8.1 | MIT OR Apache-2.0 |
| sdl2 | 0.37.0 | MIT |
| serde | 1.0.228 | MIT OR Apache-2.0 |
| thiserror | 1.0.69 | MIT OR Apache-2.0 |
| tobj | 4.0.4 | MIT |
| winres (build dep) | 0.1.12 | MIT |

Note that "MIT OR Apache-2.0" means you may pick **either** — MIT alone is
sufficient and requires only that the copyright notice travels with the code.

SDL2 is built from source as a transitive native dependency; its upstream
license (zlib) is included in its own distribution.

Because these are all Cargo dependencies rather than files you redistribute
yourself, none of them require you to ship a license file in your game's
`assets/` folder. The obligations attach to source redistributions, and your
official builds ship as compiled binaries. This file is the provenance record.

---

## Optional: Steamworks SDK

The `steam` feature is **off by default** and is not part of a default build.

Building with `--features steam` pulls in [`steamworks`](https://crates.io/crates/steamworks)
0.13.1 (MIT / Apache-2.0) and `steamworks-sys` 0.13.0. Be aware of what
`steamworks-sys` actually does: it **vendors Valve's Steamworks SDK headers**
(`lib/steam/public/steam/*.h`) and redistributable binaries inside the crate,
downloaded at build time rather than committed here.

The Rust bindings are permissively licensed, but the **SDK headers and
redistributable binaries they carry are governed by Valve's Steamworks SDK
license**, not by MIT. That license permits redistribution only as part of a
game distributed through Steam, and prohibits shipping the SDK to non-Steam
distributors.

Practical consequences for this project:

- Steam builds are fine — Steam is the sanctioned channel.
- Do **not** ship a `steam`-featured Linux build via a plain itch.io or GitHub
  download, and do not enable `steam` for the web/Wasm build. Non-Steam
  redistribution of those binaries is exactly what the SDK license forbids.
- `package.sh` already only copies `steam_appid.txt` when `FEATURES` contains
  `steam`, which keeps dev-only state out of non-Steam packages. Keep it that
  way.
- `steam_appid.txt` is a development aid. Valve's docs say not to ship it;
  `package.sh` only copies it into a Steam build, which is the one case where
  it is harmless.

---

## Updating this file

Versions above were read from the resolved dependency graph. After changing
dependencies:

```bash
cargo tree -p dreamscape --features steam
```

and refresh the table. The license expression for each crate is its `license`
field on crates.io. A useful sweep of the whole graph, not just direct deps:

```bash
cargo metadata --format-version 1 --all-features \
  | python3 -c "import json,sys; \
      print(sorted({(p['name'], p['license']) for p in json.load(sys.stdin)['packages'] \
                    if any(k in (p.get('license') or '') for k in ('GPL','MPL','CDDL','OSL','SSPL','EUPL'))}))"
```

Anything matching that grep is worth reading before you ship. As of this
writing the only hits are the 12 symphonia crates (MPL-2.0) noted above.
