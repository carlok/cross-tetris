# Cross Tetris — Milestone 2

Four-board cross Tetris (Rust engine → WASM) with a playable React UI and a
greedy rule-based AI. Mode A ("Independent Cross" per the project spec): four
standard Tetris wells arranged N/E/S/W, each fully independent — own board,
own piece queue, own hold slot. The only cross-board coupling right now is
bookkeeping: total score is the sum of the four, and the game ends when any
single arm tops out. No shared resources or garbage coupling yet.

## Project layout

```
engine/   deterministic game engine — single-board rules (rotation, gravity,
          line clears, scoring) plus CrossGame, a thin wrapper managing 4
          independent GameStates. No deps beyond std.
ai/       greedy one-ply rule-based AI, depends on engine. Same AI plays each
          arm independently (play_best_move_all), no cross-board strategy yet.
wasm/     wasm-bindgen bridge exposing engine+ai to JS, zero game logic of its
          own. WasmGame (single board) and WasmCrossGame (4-arm) both exposed.
web/      Vite + React + TypeScript UI — cross-shaped 4-board layout.
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
| 1 / 2 / 3 / 4 | select arm (North / East / South / West) |
| ← / → | move (selected arm) |
| ↑ | rotate CW |
| Z | rotate CCW |
| ↓ | soft drop |
| Space | hard drop |
| C | hold |

In human mode you control one arm at a time (switch with 1-4) while gravity
keeps advancing all four — this is the "divided attention" premise the whole
project is built to test. Click **Switch to AI** to hand all four arms to the
greedy rule-based AI simultaneously (`ai_step_all()` on an interval, one
independent placement per arm per step).

## Run the tests

```bash
cargo test --workspace                       # engine + ai, native, includes proptest
wasm-pack test --headless --firefox wasm     # wasm32-target smoke test (needs Firefox or Chrome)
```

## What's implemented

Standard SRS rotation + wall kicks, 7-bag randomizer (seeded, deterministic),
gravity/lock delay/soft/hard drop, line clears + scoring, top-out, hold — per
arm. `CrossGame` runs 4 independently-seeded arms (seeds decorrelated from one
master seed), sums their score, and ends the game when any arm tops out. A
one-ply greedy AI scores placements on aggregate height, holes, bumpiness,
height variance, and lines cleared, applied independently to each arm.

## What's not (yet)

Shared resources (global hold/action budget), garbage coupling between arms,
evolutionary optimization, replay viewer, experiment logging — all later
milestones per the full project spec.

## Known environment quirk

The in-browser game loop uses `requestAnimationFrame`, which browsers
throttle/pause for hidden or backgrounded tabs — expected behavior, not a
bug. If you're driving the page through browser automation and it appears to
"freeze," check `document.visibilityState`.
