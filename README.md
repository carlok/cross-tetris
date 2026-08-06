# Cross Tetris — Milestone 1

Single-board deterministic Tetris engine (Rust → WASM) with a minimal playable
React UI and a greedy rule-based AI. First slice of a larger project (see
`cross-tetris-rule-based-and-vectorized-spindle` plan) that will eventually
grow into a 4-arm cross-board variant with an evolutionary AI. None of that
exists yet — this is just: one board, correct rules, one baseline AI.

## Project layout

```
engine/   deterministic game engine (rotation, gravity, line clears, scoring) — no deps beyond std
ai/       greedy one-ply rule-based AI, depends on engine
wasm/     wasm-bindgen bridge exposing engine+ai to JS, zero game logic of its own
web/      Vite + React + TypeScript UI
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
| ← / → | move |
| ↑ | rotate CW |
| Z | rotate CCW |
| ↓ | soft drop |
| Space | hard drop |
| C | hold |

Click **Switch to AI** to hand control to the greedy rule-based AI instead of
the keyboard (same underlying game, same actions — it just calls `ai_step()`
on an interval instead of reading input).

## Run the tests

```bash
cargo test --workspace                       # engine + ai, native, includes proptest
wasm-pack test --headless --firefox wasm     # wasm32-target smoke test (needs Firefox or Chrome)
```

## What's implemented

Standard SRS rotation + wall kicks, 7-bag randomizer (seeded, deterministic),
gravity/lock delay/soft/hard drop, line clears + scoring, top-out, hold. A
one-ply greedy AI scoring placements on aggregate height, holes, bumpiness,
height variance, and lines cleared.

## What's not (yet)

Four-board cross layout, shared resources, garbage coupling, evolutionary
optimization, replay viewer, experiment logging — all later milestones per
the full project spec.
