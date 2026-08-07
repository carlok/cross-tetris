# Plan: boost AI strength and simulation speed (no "fancy AI")

Scope: make the existing greedy rule-based AI **play better** and make the
simulator **run faster**, using only classical, hand-derived techniques. No
neural networks, no evolutionary/genetic search, no reinforcement learning.
Those are later milestones and explicitly out of scope here.

**Read this whole file before writing code.** Do the steps in order. Each
step is independently verifiable and independently committable. Do not start
step N+1 until step N's acceptance criteria pass.

---

## Ground rules (apply to every step)

1. **Never break the existing tests.** `cargo test --workspace` must pass
   (currently 68 tests). If a change breaks a test, the change is wrong
   until proven otherwise — do not "fix" the test to match new behavior
   unless the step explicitly says behavior changes.
2. **Steps 1 and 2 must not change behavior at all.** They are pure speed
   work. Verification: the AI must pick the *exact same placements* before
   and after. Step 0 gives you the tool to prove this.
3. **Steps 3 and 4 do change behavior.** They must be measured, not assumed.
   Report before/after numbers in the commit message.
4. **Determinism is sacred.** Same seed + same agent must always produce the
   same game. Never introduce `HashMap` iteration order, `rand::thread_rng`,
   system time, or float non-determinism into a decision path.
5. **Commit after each step**, with the measured numbers in the message.
6. Do not touch `engine/src/` in steps 0–2. All work is in `ai/` and a new
   `sim/` crate. Step 3+ may add to `ai/` only.

---

## Step 0 — Benchmark harness (do this first, it makes everything else measurable)

Right now there is **no way to tell whether a change helped**. Build that
first. Every later step depends on it.

### Create a new workspace crate `sim/`

Add `"sim"` to `members` in the root `Cargo.toml`.

`sim/Cargo.toml`:
```toml
[package]
name = "sim"
version = "0.1.0"
edition = "2021"
description = "Headless batch simulator and benchmark for Cross Tetris"
license = "MIT"
publish = false

[dependencies]
engine = { path = "../engine" }
ai = { path = "../ai" }
```

`sim/src/main.rs` — a binary that:

1. Takes optional CLI args: `--seeds <start>..<end>` (default `1000..1100`),
   `--max-pieces <n>` (default `20000`).
2. For each seed, plays one full Cross Tetris game headlessly:

```rust
let mut cross = CrossGame::new(seed);
let mut pieces = 0u32;
loop {
    if cross.is_game_over() { break; }
    if pieces >= max_pieces { break; }              // capped, see WARNING
    let before = cross.total_pieces_placed();
    ai::play_best_cross_move(&mut cross, &weights);
    if cross.total_pieces_placed() == before {
        break;                                       // no progress, see WARNING
    }
    pieces += 1;
}
```

3. Records per game: `total_score()`, `total_lines_cleared()`,
   `total_pieces_placed()`, whether it ended by top-out / cap / stall.
4. After all games, prints:

```
games:            100
mean score:       ...
median score:     ...
p10 score:        ...   (worst decile — the spec cares about this)
mean lines:       ...
mean pieces:      ...
topped out:       N   capped: N   stalled: N
throughput:       ... games/sec   ... placements/sec
elapsed:          ... s
```

Use `std::time::Instant` for timing. No external crates.

> **WARNING — two ways this loop can hang, both must be guarded:**
> - **Cap:** a strong AI may survive indefinitely. Without `--max-pieces` the
>   benchmark never returns. Always cap, and report how many games hit the
>   cap (a high cap count means the AI is too strong to measure this way —
>   raise the cap or note the games are censored).
> - **Stall:** if `best_cross_placement` returns `None` (no selectable well
>   has any legal placement) `play_best_cross_move` is a silent no-op and the
>   loop spins forever. The `total_pieces_placed()` no-progress check above
>   catches it. Do not remove that check.

### Seed hygiene (required by the project spec)

- **Dev seeds `1000..1100`** — use these while iterating on steps 1–4.
- **Test seeds `5000..5100`** — touch these **only** for the final
  before/after report. Never tune against them.

Keep both ranges in the README table you produce at the end.

### Also add: a placement-equivalence check mode

Add `--compare-baseline` (or a separate small binary/test) that, for each dev
seed, records the **full sequence of chosen placements** (`arm`, `rotation`,
`column`, `row`) as a `Vec<CrossPlacement>`. Steps 1 and 2 must produce a
byte-identical sequence to the pre-change baseline. Save a baseline file
(e.g. `sim/baseline_placements.txt`) and diff against it.

This is how you prove "no behavior change" instead of hoping.

### Acceptance criteria for step 0

- [ ] `cargo run -p sim --release` completes and prints the stats block.
- [ ] Running it twice with the same seeds gives **identical** output
      (determinism check).
- [ ] Baseline numbers for dev seeds recorded in the commit message.
- [ ] `cargo test --workspace` still passes.

> Run the simulator with `--release`. A debug build is 10–50× slower and will
> mislead you about throughput.

---

## Step 1 — Cheap speed wins in the AI hot path (no behavior change)

Two obvious inefficiencies in `ai/src/greedy.rs`. Both are mechanical.

### 1a. Replace the row-by-row drop loop with a direct landing calculation

Current `enumerate_placements` starts each candidate at `row: -40` and steps
down one row at a time, calling `piece_fits` each time — roughly **80
`piece_fits` calls per candidate**, and there are ~40 candidates per well.

Replace with the standard "skirt" technique, which computes the landing row
in O(4):

- For the board, compute `col_top[c]` = the **smallest** engine row index `r`
  where column `c` is filled, or `BOARD_TOTAL_HEIGHT` (40) if the column is
  empty. (Remember: row 0 is the top, row 39 is the bottom.)
- For the piece shape, compute `skirt[dc]` = the **largest** `dr` among the
  piece's cells having that `dc` (i.e. the lowest cell of the piece in that
  column of its bounding box).
- The landing row is:
  `row = min over each occupied dc of ( col_top[c0 + dc] - skirt[dc] - 1 )`
  where `c0` is the piece's board column.

This is exact for a straight-down drop (which is all this function does), and
it handles overhangs correctly because `col_top` is the topmost filled cell.

> **Verify, don't trust:** temporarily keep the old loop and `debug_assert!`
> that both produce the same row for every candidate across the dev seeds.
> Remove the old loop only once that assertion has run clean over a full
> benchmark pass.

### 1b. Stop heap-allocating per decision

- `enumerate_placements` returns `Vec<ActivePiece>`, allocated fresh for
  every well on every decision. There are at most 4 rotations × 10 columns =
  **40 candidates**, so use a fixed-size array + length, or write into a
  caller-supplied reusable buffer (`&mut Vec<ActivePiece>` that the caller
  clears and reuses).
- `simulate_lock` calls `board.clone()` — a heap allocation of a
  `Vec<Option<PieceKind>>` — **once per candidate**, so ~160 allocations per
  cross decision. Step 2 removes this entirely; if step 2 is deferred, at
  minimum reuse one scratch board.

### Acceptance criteria for step 1

- [ ] `cargo test --workspace` passes.
- [ ] `--compare-baseline` shows an **identical** placement sequence to the
      step-0 baseline (this is the whole point — zero behavior change).
- [ ] Throughput improved. Record before/after `placements/sec`.

---

## Step 2 — Bitboard for the AI's search only (no behavior change)

The single biggest speed win. **Do not change `engine::Board`** — it stores
`Option<PieceKind>` because rendering needs the piece colors, and the engine
and all 68 tests depend on its exact API. Losing color would break rendering.

Instead add a **search-only** bitboard inside the `ai` crate.

### Create `ai/src/bitboard.rs`

```rust
/// Colorless, allocation-free board used only inside the AI's search.
/// One u16 per engine row (bits 0..=9 = columns 0..=9), rows[0] = top.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct BitBoard {
    rows: [u16; 40],   // BOARD_TOTAL_HEIGHT
}
```

Because it is `Copy` and 80 bytes, "cloning" it per candidate is a register/
stack copy with **zero heap allocation** — replacing the current `Vec` clone.

Implement:
- `from_board(&engine::Board) -> BitBoard` — called **once per well per
  decision**, not once per candidate. This is the only conversion cost, and
  it is amortised over ~40 candidates.
- `is_row_full(r)` → `rows[r] == 0b11_1111_1111` (0x3FF)
- `clear_full_rows(&mut self) -> u32` — same semantics as
  `engine::Board::clear_full_rows`.
- `col_top(c) -> usize` and/or a `col_tops() -> [usize; 10]`.
- `place(piece_mask)` — OR the piece's row masks in.

### Precompute piece masks

For each `(PieceKind, Rotation)` precompute the 4 cells once, then for a
given column produce up to 4 `(row_offset, u16)` pairs by shifting. Build
these as `const`/`static` tables or a `OnceLock`-free lazy static array — do
**not** recompute shapes inside the candidate loop.

### Rewrite the AI's evaluation to work on `BitBoard`

`simulate_lock`, `count_holes`, `column_heights`, and `extract_features`
all move to bitboard operations. Bit tricks that help:
- column heights: `40 - (index of first row whose bit c is set)`
- holes in column c: count set bits below the column top that are **not** set
  — i.e. walk rows from `col_top(c)+1` to 39 and count where bit c is clear.

### Acceptance criteria for step 2

- [ ] `cargo test --workspace` passes (all 68 — the engine is untouched).
- [ ] Add a **differential test** in `ai/tests/`: for many random boards,
      assert `BitBoard::from_board(&b)` agrees with `engine::Board` on
      `column_height` for every column, on `is_row_full` for every row, and
      that `clear_full_rows` returns the same count and yields an equivalent
      board. This is what proves the bitboard is faithful.
- [ ] `--compare-baseline` shows an **identical** placement sequence.
- [ ] Record the throughput multiplier vs. step 1.

---

## Step 3 — Stronger features (behavior changes; measure it)

The current feature set is weak: aggregate height, holes, bumpiness, height
variance, lines cleared. Replace it with **Dellacherie's** six features — a
classical hand-derived set (no learning involved), widely reported as far
stronger than the height/bumpiness family.

### The six features — define these precisely, they are easy to get wrong

All computed on the board state **after** the piece is locked, except
`landing_height` and `eroded_piece_cells` which need placement information.

1. **`landing_height`** — the height above the floor of the placed piece,
   measured **before** line clears. Use the mean row of the piece's 4 cells:
   `landing_height = mean over the 4 cells of (40 - r)`, so the bottom row
   (r=39) has height 1.

2. **`eroded_piece_cells`** = `(lines cleared) × (how many of the placed
   piece's own 4 cells were in those cleared rows)`. You must count this
   during `simulate_lock`, before the rows collapse. If nothing clears, it is 0.

3. **`row_transitions`** — for each row, scan columns left to right treating
   **both side walls as filled**, and count adjacent pairs whose filled/empty
   state differs.
   > **CRITICAL:** only count rows from the **highest filled row down to the
   > bottom**. A completely empty row contributes 2 transitions (wall→empty,
   > empty→wall), so including the 20 empty hidden rows adds a term that
   > *rewards building higher* — the wrong sign. Skipping empty rows above
   > the stack is the standard fix. Get this wrong and the AI plays worse
   > while the code looks correct.

4. **`column_transitions`** — same idea vertically, per column, with the
   **floor treated as filled** and everything above the stack treated as
   empty. Same "start at the stack top" rule as above.

5. **`holes`** — empty cells with at least one filled cell somewhere above
   them in the same column. (The existing `count_holes` already does this.)

6. **`well_sums`** — for each empty cell whose **left and right neighbours
   are both filled** (walls count as filled), measure the depth `d` of the
   contiguous run of empty cells continuing downward in that column, and add
   `1 + 2 + ... + d`. Iterate top-down and skip cells already counted as part
   of a run above them, so each well is counted once.

### Weights

Use Dellacherie's published weights **as a complete set** — they are tuned
for exactly these six features and their exact definitions:

```rust
pub const DELLACHERIE_WEIGHTS: Weights = Weights {
    landing_height:      -4.500158825082766,
    eroded_piece_cells:   3.4181268101392694,
    row_transitions:     -3.2178882868487753,
    column_transitions:  -9.348695305445199,
    holes:               -7.899265427351652,
    well_sums:           -3.3855972247263626,
};
```

> Do **not** mix these with the old features (aggregate height, bumpiness,
> variance) and keep these weights. The weights are only meaningful as a set
> with these definitions. If you want both sets available, keep them as two
> separate, independently-weighted evaluators and pick between them at the
> call site.

Keep the old weights/features available behind the existing `Weights` type or
a small enum so the benchmark can compare old vs. new directly. The old
agent is the baseline you are measuring against — do not delete it.

### Acceptance criteria for step 3

- [ ] `cargo test --workspace` passes. Existing AI tests that assert
      *specific placements* may legitimately change — if one does, verify by
      hand that the new placement is sensible and update the test with a
      comment explaining why, rather than deleting the test.
- [ ] Add unit tests for each of the six features on small hand-built boards
      with hand-computed expected values. Especially `row_transitions` and
      `well_sums` — those are the two that silently go wrong.
- [ ] Benchmark on **dev seeds**: report old vs. new median/mean/p10 score
      and lines. Expect a large improvement; if it is flat or worse, a
      feature definition is wrong (check the empty-row rule in #3/#4 first).

---

## Step 4 — Two-ply lookahead using the shared queue (behavior changes; measure it)

The AI currently sees only the piece it is placing, and throws away
information the game already gives it. `CrossGame::next_queue(n)` exposes the
upcoming pieces from the shared bag — **use the next one**.

### The search

For the current piece and the next piece:

```
for each first placement P1 (over all selectable wells):
    board1 = result of applying P1
    best_reply = max over all second placements P2 on board1 of eval(board2)
    total(P1) = eval(board1) + best_reply
pick P1 with the highest total
```

Note the second piece may go into **any selectable well**, not just the one
P1 used — that is the whole point of the shared queue, and it is what lets
the AI reason about balancing the cross.

### Cost control — this is why steps 1–2 came first

Naively this is ~160 × 160 ≈ 25,600 simulations per decision. Add **beam
pruning**: score all first placements at 1-ply, keep only the top `K`
(default `K = 8` or `10`, make it a parameter), and run the second ply only
for those. That is ~10 × 160 = 1,600 simulations — comfortable with the
step-2 bitboard.

Make the depth and beam width parameters so the benchmark can compare
1-ply vs. 2-ply-K8 vs. 2-ply-K20 and you can pick the knee of the curve.

### Also model selectability at ply 2

After P1, the piece counts change, so the `MAX_WELL_IMBALANCE` rule may block
a well for the second piece. Respect `is_well_selectable` at ply 2 —
otherwise the AI plans replies it will not be allowed to make.

### Acceptance criteria for step 4

- [ ] `cargo test --workspace` passes.
- [ ] Determinism holds: ties must break deterministically (stable iteration
      order, no float NaN comparisons — `partial_cmp().unwrap()` will panic
      on NaN, so make sure no feature can produce NaN).
- [ ] Benchmark on dev seeds: 1-ply vs. 2-ply at a few beam widths. Report
      score gain **and** the cost in placements/sec.
- [ ] Watch the cap/stall counters — if 2-ply pushes most games into
      `--max-pieces`, the agent has outgrown the benchmark; raise the cap and
      say so in the report.

---

## Step 5 — Optional extras (only if steps 0–4 are done, committed, and measured)

Lower or less certain ROI. Do these one at a time, measuring each.

- **Use the hold slot.** Each well has its own hold. Adds a branch to the
  search (place current vs. swap with hold), roughly doubling cost. Moderate
  gain, real complexity — measure before keeping.
- **Cross-global safety features.** Add features over the *whole cross*, not
  just the well being played: max stack height across wells, variance of
  heights, count of wells near top-out. This targets the actual failure mode
  (the game ends when **one** well tops out, so the AI should avoid letting
  any single well get dangerous even if the total looks fine). Genuinely
  cross-specific and a good fit for this project.
- **Avoid the imbalance trap.** Penalise placements that leave the AI with a
  bad well blocked-and-forced later. Partly handled by 2-ply already; measure
  whether an explicit term adds anything.

---

## Final report (required)

Once steps are done, run the **test seeds `5000..5100`** — once — and add a
results table to the README:

| Agent | Median score | Mean | p10 | Mean lines | Placements/sec |
|---|---|---|---|---|---|
| baseline greedy (1-ply, old features) | | | | | |
| + Dellacherie features | | | | | |
| + 2-ply beam K=8 | | | | | |

State the seed ranges, the piece cap, and how many games hit the cap. Do not
report dev-seed numbers as final results.

---

## Pitfalls, ranked by how likely they are to bite

1. **Measuring in a debug build.** Always `--release` for any timing claim.
2. **The empty-row bug in row/column transitions** (step 3). Silently makes
   the AI worse while looking correct.
3. **Infinite benchmark loop** — a strong AI never tops out, or the search
   returns `None` and the loop spins. Both guards in step 0 are mandatory.
4. **Claiming a speedup without proving identical behavior.** Steps 1 and 2
   must produce identical placement sequences. If they do not, you have a
   bug, not an optimization.
5. **Mixing feature sets with mismatched weights** (step 3). The Dellacherie
   weights are a package deal.
6. **Tuning against the test seeds.** Use dev seeds while iterating; touch
   the test seeds once, at the end.
7. **Breaking determinism** via `HashMap` ordering, unstable sorts on equal
   keys, or NaN in `partial_cmp`.
8. **Optimizing before measuring.** Step 0 exists so every later claim is
   backed by a number.
