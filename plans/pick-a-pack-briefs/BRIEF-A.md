# Stream A: pack rules (P1 + pity part of P3). Worktree: /home/_l/dreamscape-wt/a  Branch: feat/pick-a-pack-a
Files you own: src/packs.rs, and small additions in src/lottery.rs / src/cards.rs. Do NOT edit main.rs, hud.rs, reveal_ui.rs, pack_art.rs.

Implement per plan "Design":
1. `offer_packs(log, owned, boost)`: group `log` by theme; rank themes deepest first (max depth, tie-break stable);
   take up to MAX_OFFERS. Each pack = that theme's records (cap MAX_PACK_CARDS, keep deepest/most recent consistently),
   cards built with the existing `cards::recall` logic (reuse it, per-theme slice) so rarity/remembered rolls are identical to today.
   `memory_chances[i]` = `cards::memory_chance(card.rarity, card.depth, boost)` exactly (same value used for the roll).
   Awakening (waking dream) stays remembered as today and goes in its own theme's pack.
   Empty log -> empty vec. Non-empty log -> at least one pack.
2. First-run safety: if `owned` is empty, ensure the offer includes a pack with a playable (non-Awakening) remembered card:
   the Garden pack if the run has Garden, else the first playable. Reuse the logic idea of `shape_first_pack`
   (apply it so the guarantee holds across offered packs; do not break its existing tests).
3. Fused/Resonant: only via a record's `blend` (already handled by `card_from`). Nothing in packs.rs may inject them.
4. `Emblem::for_theme(theme, seed)`: deterministic; every Emblem reachable across themes; Awakening/Lobby sensible (Eye).
5. `seed` per offer: deterministic from the theme + run records so replays match.
6. Pity: add to lottery.rs a fn `apply_pack_pity_to_offer(state, offer: &mut PackOffer)` (thin wrapper over existing
   `apply_pack_pity` on `offer.cards`) and a doc comment that it must be called only for the OPENED pack. Counter must not
   move for unopened packs (nothing in offer_packs touches LotteryState).
Tests (failing first), all in packs.rs/lottery.rs: distinct themes across offers; <3 themes -> fewer; empty log -> none;
first-run guarantee (empty booklet always gets a playable remembered card, Garden preferred); chosen pack's cards == run's dreams of
that theme (cap 5); 10k random runs never yield Fused/Resonant unless source record has `blend`; memory_chances equal memory_chance;
emblems deterministic + all reachable; pity counter unchanged by offer_packs and advances only through the wrapper.
