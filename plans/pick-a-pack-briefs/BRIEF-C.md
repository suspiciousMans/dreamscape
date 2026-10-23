# Stream C: pick screen + wiring (P2 + rest of P3). Worktree: /home/_l/dreamscape-wt/c  Branch: feat/pick-a-pack-c
Files you own: src/reveal_ui.rs (pick screen drawing/layout), src/hud.rs (Mode + dispatch), src/main.rs (state, input, wiring). Not packs.rs/pack_art.rs/lottery.rs internals.
Streams A and B are filling `packs.rs` and `pack_art.rs` at the same time: code against the contract signatures only. Until merge, `offer_packs` panics (`todo!`),
so your tests must build `PackOffer`s by hand. Never call todo!() paths in your tests.

Build:
1. `hud::Mode::PackPick` + `reveal_ui::PackPickView { offers: Vec<PackOffer>, selected: usize, age: f32 }` (Default).
2. `draw_pick(ctx, p, screen, v, view, packs_art: &mut PackArt, card_art)`: title (reuse the glitch_text style of draw_reveal), up to 3 `pack_art::draw_pack` wrappers
   side by side at the selected one slightly raised/scaled, under each: theme name, "N CARDS", and the per-card memory chances (from `offer.memory_chances`, shown exactly as % of
   `memory_chance`, e.g. "REMEMBERED 60% / 40%"). Also show the lottery pity counter line if lottery state exposes one (read-only). Footer: "[A/D] choose  [ENTER] open".
   Tilt toward cursor/selection, scaled by motion; reduced motion = static.
3. Flow: Summary (Return/Space) now calls `start_pack_pick()` instead of `open_pack()`: builds `offer_packs(&run_log, &owned, boost)` (same boost as old `open_pack`) -> `Mode::PackPick`.
   If the offers vec is empty, fall back to straight to existing reveal with an empty pack (same as today's behaviour for empty logs).
   A/D/Left/Right/1-3/mouse hover+click/pad move the selection; Return/Space/click opens: sets `self.pack = PackView{ pack: offer.cards.clone(), .. }`,
   applies `lottery::apply_pack_pity_to_offer`-equivalent on the OPENED pack only (A is adding `apply_pack_pity_to_offer(state, &mut PackOffer)` in lottery.rs; if it isn't merged yet,
   call the existing `lottery::apply_pack_pity(&mut self.booklet.lottery, &mut cards)` on the chosen cards — same behaviour), then `Mode::Reveal`. Unchosen offers are dropped (cleared from state).
   Everything downstream (reveal flip, `save_journal`/`Booklet::press`, dust, summary of dust, achievements) stays unchanged and works on the chosen pack only.
   Keep the `DREAMSCAPE_PACK_PREVIEW` dev switch working (padded log -> offers).
4. Mode plumbing everywhere `Mode::Reveal` is matched (hud dispatch, input, pad, pause, settings back-nav, co-op if it touches it: solo path only, co-op out of scope, but do not break it).
5. Tests (failing first): selection wraps and clamps; keyboard/pad choose; picking sets `self.pack.pack` == chosen offer cards and drops others; unchosen cards never reach the booklet
   (press only sees chosen); pity advances only on chosen pack; empty-offer fallback; `screen_smoke` draws `Mode::PackPick` for 1, 2 and 3 offers (hand-built offers) at 1280x720 and small size;
   reduced motion => no animation difference between frames. Existing reveal and screen_smoke tests must still pass.
