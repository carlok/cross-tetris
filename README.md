# Cross Tetris — Milestone 2

Four-well cross Tetris (Rust engine → WASM) with a playable React UI and a
greedy rule-based AI. Mode A ("Independent Cross" per the project spec): four
standard Tetris wells arranged N/E/S/W, **sharing one piece stream**. Only one
piece is ever falling at a time — you (or the AI) pick which well it goes to,
then it plays out as ordinary real-time Tetris there until it locks, and the
next queued piece needs a well picked again. The four wells are locked stacks
waiting their turn, not four simultaneous independent games. Total score is
the sum of the four, and the game ends when any single well tops out. No
shared resources or garbage coupling yet.

## Project layout

```
engine/   deterministic game engine — single-board rules (rotation, gravity,
          line clears, scoring) in game.rs (still usable standalone), plus
          cross.rs: CrossGame, one shared 7-bag feeding a single falling
          piece that gets routed to one of 4 wells at a time. No deps beyond
          std.
ai/       greedy one-ply rule-based AI, depends on engine. best_cross_placement
          evaluates the upcoming piece against all 4 wells at once and picks
          one (arm, rotation, column) — the AI's whole decision is which well
          plus where in it, per spec section 4.1.
wasm/     wasm-bindgen bridge exposing engine+ai to JS, zero game logic of its
          own. WasmGame (single board) and WasmCrossGame (4-well cross) both
          exposed.
web/      Vite + React + TypeScript UI — cross-shaped 4-well layout.
```

## Prerequisites

- Rust + Cargo (`rustup target add wasm32-unknown-unknown`)
- [`wasm-pack`](https://rustwasm.github.io/wasm-pack/) (`cargo install wasm-pack`)
- Node.js + npm

## Run it

```bash
# 1. build the wasm bridge (regenerate after any Rust change)
wasm-pack build wasm --target web

# 2. install web deps (first time only)
npm --prefix web install

# 3. start the dev server
npm --prefix web run dev
```

Open the printed local URL. Controls:

| Key | Action |
|---|---|
| Arrows or 1/2/3/4 | send the next queued piece to North/East/South/West (arrows match the layout: Up=N, Right=E, Down=S, Left=W) |
| ← / → | move the currently falling piece |
| ↑ | rotate CW |
| Z | rotate CCW |
| ↓ | soft drop |
| Space | hard drop |
| C | hold (per-well hold slot) |

Arrow keys double as well-selection and movement — never ambiguous, since a
piece only starts falling after a well is picked, so the two meanings never
overlap in time. Movement keys always act on whichever piece is currently
falling (there's only ever one). If you don't pick a well within
`SELECTION_TIMEOUT_MS` (5s, `engine/src/cross.rs`), one is chosen at random
from a deterministic RNG stream — shown as a countdown bar under the next-
piece preview. Click **Switch to AI** to hand the queue to the greedy
rule-based AI, which evaluates all four wells for each piece and routes it
to the best one (`ai_step()` on an interval, one placement per step).

## Run the tests

```bash
cargo test --workspace                       # engine + ai, native, includes proptest
wasm-pack test --headless --firefox wasm     # wasm32-target smoke test (needs Firefox or Chrome)
```

## What's implemented

Standard SRS rotation + wall kicks, 7-bag randomizer (seeded, deterministic),
gravity/lock delay/soft/hard drop, line clears + scoring, top-out, hold — per
well. `CrossGame` holds one shared bag and routes its single active piece to
whichever well is selected; each well keeps its own board/score/level/hold/
top-out flag. A one-ply greedy AI scores every (well, rotation, column)
combination for the upcoming piece on aggregate height, holes, bumpiness,
height variance, and lines cleared, and picks the argmax across all four
wells at once.

## What's not (yet)

Shared resources beyond the implicit single queue (global action budget),
garbage coupling between wells, evolutionary optimization, replay viewer,
experiment logging — all later milestones per the full project spec.

## Known environment quirk

The in-browser game loop uses `requestAnimationFrame`, which browsers
throttle/pause for hidden or backgrounded tabs — expected behavior, not a
bug. If you're driving the page through browser automation and it appears to
"freeze," check `document.visibilityState`.
