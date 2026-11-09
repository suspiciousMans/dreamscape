# Brief: Scraps S1-S4
Goal: implement `plans/PLAN-scraps.md` end to end, in order S1 -> S4, committing after each green milestone.

Read first: PLAN-scraps.md, CONTEXT.md here, `deck.rs` toolcards block, `main.rs::open_pack`, `moth_shop.rs` tool offer, `loadout_ui.rs`, `hand.rs`.

S1. `scraps.rs`: `ScrapKind` (6 kinds, `ALL_SCRAPS`), `roll_scrap(seed) -> ScrapKind`, `Satchel` with `add`, `count`, `spend`, `carry(limit)`; `SATCHEL_CARRY = 2`. Add `Booklet.satchel` with `#[serde(default)]`. Tests: determinism, spend at zero, old-save load.
S2. `deck::SCRAP_BASE` = `TOOL_BASE - 16`; `scrap_id`, `scrap_kind`, `is_scrap`; shuffle carried scraps into the draw pile; playing one exhausts it (not discarded back); `SavedRun::deck` saves/restores them (serde default). Tests: id ranges never collide, exhaust, resume.
S3. Effects, using existing levers: Mend refills nerve; Lull calms enemies for ~6 s; Lantern reveals the nearest shard; Windfall adds dust at run end; Safe Step cancels the next nerve hit; Redraw discards the hand and draws fresh. Drive each in a test with bare entities where possible (no GL). Spend from the satchel on play.
S4. UI: pack reveal shows the scrap gained after the last card; loadout lets the player pick up to 2 carried scraps; the hand draws a scrap as a small card with a short label. Painters through `pixel_ui` run-merged helpers. `screen_smoke` at 640x360 and 1280x720; check `audit_text_fits_every_screen` and the UI budget test.
Exclusions: co-op, autopilot, old-format resumed runs carry none.
Approval list: names and descriptions in `plans/scrap-briefs/NOTES-scraps.md` under "for approval".
Run: `cargo test -p dreamscape -- scraps deck screen_smoke audit_text` focused, full suite once at the end in the background with notify.
