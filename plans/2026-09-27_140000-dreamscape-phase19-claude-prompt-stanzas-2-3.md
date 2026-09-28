# Dreamscape Phase 19, Stanzas 2 and 3 — Claude build prompt

This is a plan, not lore. Treat it like a spec a senior Rust game engineer wrote for you: follow it step by step, don't reinterpret it, and ask before changing scope.

## Where you are
- Repo root: `C:/Users/james/jame_inspect` (git-bash on Windows, CRLF; `cargo … -p dreamscape` from the repo root).
- Branch you're on is one you just created for this; do not rebase onto main mid-build.
- Game crate: `games/dreamstage/src/`. The plan's paths are relative to that `src/` unless they say otherwise.
- `cargo test -p dreamscape 2>&1 | grep "test result" | head -1` currently shows **321 passed, 2 ignored** (the baseline after PR #11).
- Real booklet lives at `games/dreamscape/booklet.ron`. Never point a test run at it. Use `DREAMSCAPE_BOOKLET="$LOCALAPPDATA/Temp/<temp>.ron"` for every E2E run and screenshot.
- Use the dev-switch gate `crate::dev::var`. Release builds ignore switches.
- Commit after each task that turns tests green: `git add games/dreamscape && git commit -qm "dreamscape: <what>"`.

## What's already in the tree (don't redo these)
- `lore.rs`, `eye.rs`, `objective.rs`, `tutorial.rs`, `memories_ui.rs`, `dream::Goals`, fragments + note fields, the note/end/memories screens, the prologue, and `DreamDirector.forced`.
- Fused-rarity fields and lore fields already in `Booklet`/`Card`/`DreamRecord`/`names.rs`.
- Story texts are already written in `lore.rs`. Do not rewrite them. The plan's tone rule for the story (already in tree): gentle, specific, hopeful at the end; never method, never self-harm.

## What you build now

### Stanza 2 — cards with power (Parts F–G of the plan)

Read the relevant existing code first (`grep`/`cargo test` are your friends):
- `cards.rs`: `Rarity`, `Attribute`, `Card`, `DreamRecord`, `Booklet`, numbering (`add_run` and `press`).
- `upgrades.rs`: `Ability` (9 kinds), `passives`, `RunUpgrades`, stat getters, `roll_choices`.
- `dream/theme.rs`: `ALL_THEMES` (20) and `ThemeSpec` fields.
- `dream/names.rs`: `dream_name`, `fused_name` (already exists? check).
- `main.rs`: current `load_dream` path, `begin_run`, `start_from_title`, which `Rarity` gets pressed for a dream (already pressed from dreams, check).

Deliver, in this order:

**1. `powers.rs`** — every attribute has a power.
- Each card's attribute gives it an **active** ability (one of the 9 existing), a **passive** upgrade (one of the 14 existing), and that dream's first accent colour as `tint`.
- A flavour line: one sentence in that dream's voice, short and readable.
- Tests: for every theme, the power exists and every ability is reachable by some card, and the tint equals `theme.spec().accents[0]`.
- Tone: keep flavour lines short, dry, game-y. Nothing preachy.

**2. `Rarity::Fused` and `Rarity::Resonant`** lives in the plan.
- Fused: label "FUSED", `base_memory` 1.0, dust 30. Colour iterates between the two themes' accents.
- Resonant: label "RESONANT", `base_memory` 1.0, dust 45. Colour = `hue(time * 0.8)` with a white pulse.
- Add `fused: Option<DreamTheme>` to `Card`. Add `blend: Option<DreamTheme>` to `DreamRecord`.
- These two rarities never come from `from_score`, and `bumped()` leaves them unchanged. Test: 10k random scores → never Fused/Resonant; round-trip a Card with `fused: Some(Garden)` through RON.
- If `card_from` already assigns rarity from the dream record, set Fused when `r.blend.is_some()` (that's the plan's rule — don't invent a new rule).

**3. Loadout rules (the whole point of Stanza 2).**
- Each chosen loadout card brings in: its active as one ability-slot entry (slots 1–2 fill the existing ability slots, later cards add passives only), its passive stack, its fused passive if any, and +1 passive if it's a Resonant.
- Cards replace upgrade-pick abilities. Passive picks between dreams stay.
- Tests: `from_loadout` on 1 card gives 1 active + 1 passive; 3 cards give 2 actives + 3 passives; no duplicate active learned twice; passives still count toward caps so picks won't offer a maxed passive.
- Update existing upgrade tests that assert abilities appear in `roll_choices`: remove the ability part, add `abilities_come_from_cards_not_picks`.

**4. Card dreams appear on a schedule (the user's ask: spaced out).**
- A card's dream turns up at depth `3 + 3*i` (3, 6, 9, 12, 15) — not every two depths.
- If that depth is a nightmare, slide one dream later. Never two fence-card dreams back to back.
- Each planned card dream also sets `director.blend` to the card's `fused` theme, if the card was fused.
- Test: plan 5 themes, descend 20 times, each planned theme appears exactly once at its depth or one later, never on a nightmare, never twice in a row.

**5. Loadout screen (`Mode::Loadout`).**
- Title **START** now goes to the loadout screen, unless you have 0 cards.
- Layout: rows for unlocking slots; card grid in a scroll; selected card's panel shows name, dream, active (`SHIFT: DASH` or "(passive only)"), `+ SWIFT FEET`, flavour line.
- Keys: `[a/d/w/s] browse   [enter] add/remove   [space] fall asleep   [esc] back`. Pad: d-pad, A = Return, X = Space, B = Escape.
- Persist `lore.loadout`. Cards no longer in the booklet drop out silently.
- Dedup slot keymaps if the plan already has one (check before adding a new `SLOT_KEYS` constant).
- A card in the loadout gets its dream guaranteed once per run.

**6. Card visuals.**
- Each ability's flash uses the tint of the loadout card that granted it.
- An ability burst: 8 small cubes in a ring at the player, tinted by the card's dream colours, fading over 0.5 s.

### Stanza 3 — merging, fused dreams, collected dreamers, the ending (Parts G–I)

**7. `fusion.rs`: merging cards.**
- Merging uses up both cards and makes one Fused card: the first card's shape, the second card's colours and mood.
- 10 resonant recipe pairs (order doesn't matter). On a resonant pair, the new card is Resonant instead of Fused.
- Resonant cards are defined in the plan — don't invent new recipe pairs without telling the user.
- Every RON path round-trips; `b.runs[0]` isn't empty when cards exist.

**8. Natural fused dreams.**
- At depths past the first few, a fuse can happen naturally: rate grows slowly with depth, capped.
- When it happens, `director.blend` is set for that run; that dream presses a Fused card, and the eye speaks `FusedDream`.
- Test: over 3000 descents, fused rate is between 4% and 16%, and never happens on a nightmare dream.

**9. Collected dreamers (boon + burden).**
- A dreamer is collected when every one of their notes is found (already tracked in `lore.dreamers`).
- The loadout screen gets a companion slot, unlocked by the first collected dreamer.
- A companion gives a boon (existing passives) and a burden (a debuff from their weight). Each burden applies to one stat: boom slows movement, anxiety makes enemies notice you sooner, guilt shortens grace, insomnia speeds enemies up, heartbreak lowers jump, grief lowers sight, pressure reduces shard bonuses, loneliness removes a card slot at each pick.
- Tests: every weight gives a boon and a burden, and each burden hits the stat it names.

**10. The ending.**
- Once you've found all 12 of your own notes, a dream at the bottom happens.
- The eye speaks 6 short lines over 30 seconds.
- After the ending: the eye closes (lid animation), credits text, helpline line, then continue.
- The title screen marks the game complete. Runs continue after; notes of others still wait.

### Testing and verification rules (non-negotiable)
- Follow the TDD cycle per task: failing test, red, implement, green, commit.
- Unit tests before E2E. Run `cargo test -p dreamscape` after each big step.
- E2E runs: seeds 1–5 with autopilot, plus forced-stanza tests for card dreams and fused dreams.
- Every E2E run uses its own temp booklet.
- If a forced-stanza test doesn't trigger, widen the run before suspecting the logic. E.g. for card-dream forcing, use 20 descents; for natural fuses, use 3000 descents.
- Screenshots for Stanza 2: loadout screen, an ability flash, a card's detail. For Stanza 3: a merged card, a fused dream, the ending.

### Don't do
- Don't touch the real booklet in tests or screenshots.
- Don't change story-tone until the user plays and responds. The plan already has the tone in tree.
- Don't add new story text in Stanza 2/3; story text belongs to Stanza 1's lore, and you already have it.
- Don't rebase the branch onto main mid-build unless asked.

### Ask before
- Adding new abilities or new UI modes not in the plan.
- Changing `Difficulty` or `Pressure` code (those affect balancing and existing tests).
- Changing dream generation layout RNG (existing seeds must replay).
