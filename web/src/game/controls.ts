import { WasmArm } from '../wasm'

/**
 * Screen-relative control scheme, shared by keyboard and gamepad input.
 *
 * Board.tsx's per-arm rendering transform means "engine move_left" (which
 * always decreases `col`) doesn't always move the piece toward the left of
 * the SCREEN — for a landscape well (East/West) `col` maps to the screen's
 * vertical axis, and for North the whole board is rendered rotated 180°.
 * Binding ArrowLeft to engine move_left unconditionally (as a first pass
 * did) means the controls only feel right for South, which happens to be
 * the identity transform.
 *
 * Instead, each arrow key is given a fixed *physical* meaning — "the key
 * pointing toward this well's fall direction accelerates the fall; the
 * opposite key rotates; the two keys perpendicular to the fall direction
 * move the piece side to side" — and this module resolves that physical
 * meaning to the correct engine action for whichever arm is currently
 * active. South needs no remapping under this scheme (gravity points down,
 * so Down/Up/Left/Right already line up with soft-drop/rotate/move as
 * before); the other three are derived from Board.tsx's actual transform:
 *
 *   North (180° rotation):        Up=soft-drop, Down=rotate, Left/Right swapped
 *   West  (90° CW, landscape):    Left=soft-drop, Right=rotate, Up/Down move
 *   East  (90° CCW, landscape):   Right=soft-drop, Left=rotate, Up/Down move
 */
export type Semantic = 'moveLeft' | 'moveRight' | 'softDrop' | 'rotateCw'
export type ArrowKey = 'up' | 'down' | 'left' | 'right'

const SCHEME: Record<WasmArm, Record<ArrowKey, Semantic>> = {
  [WasmArm.South]: { up: 'rotateCw', down: 'softDrop', left: 'moveLeft', right: 'moveRight' },
  [WasmArm.North]: { up: 'softDrop', down: 'rotateCw', left: 'moveRight', right: 'moveLeft' },
  [WasmArm.West]: { up: 'moveLeft', down: 'moveRight', left: 'softDrop', right: 'rotateCw' },
  [WasmArm.East]: { up: 'moveRight', down: 'moveLeft', left: 'rotateCw', right: 'softDrop' },
}

export function resolveDirection(arm: WasmArm, key: ArrowKey): Semantic {
  return SCHEME[arm][key]
}
