// Thin re-export isolating the rest of the app from the generated
// wasm-pack output path (`../../wasm/pkg`, built via `wasm-pack build
// --target web` from the repo root's `wasm` crate).
import init, { WasmGame } from '../../../wasm/pkg/cross_tetris_wasm.js'

let initPromise: Promise<unknown> | null = null

export function initWasm(): Promise<unknown> {
  if (!initPromise) {
    initPromise = init()
  }
  return initPromise
}

export { WasmGame }
