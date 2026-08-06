import { useEffect, type RefObject } from 'react'
import { WasmArm, type WasmCrossGame } from '../wasm'

// Direct 1:1 keydown -> action mapping, no DAS/ARR (auto-repeat) tuning in
// this milestone (documented follow-up). Movement actions always target
// whichever piece is currently falling — there's only ever one, so no arm
// targeting is needed (the engine no-ops safely if nothing is falling).
const MOVE_ACTIONS: Record<string, (g: WasmCrossGame) => void> = {
  ArrowLeft: (g) => g.move_left(),
  ArrowRight: (g) => g.move_right(),
  ArrowUp: (g) => g.rotate_cw(),
  KeyZ: (g) => g.rotate_ccw(),
  ArrowDown: (g) => g.soft_drop_start(),
  Space: (g) => g.hard_drop(),
  KeyC: (g) => g.hold(),
}

// Number keys 1-4 commit the next queued piece to a well.
const DIGIT_WELL_KEYS: Record<string, WasmArm> = {
  Digit1: WasmArm.North,
  Digit2: WasmArm.East,
  Digit3: WasmArm.South,
  Digit4: WasmArm.West,
}

// Arrow keys double as well selection while awaiting one — spatially
// matching the cross layout (Up=North, Right=East, Down=South, Left=West).
// No conflict with their movement meaning: a piece is only ever falling
// once a well has already been picked, so the two uses never overlap.
const ARROW_WELL_KEYS: Record<string, WasmArm> = {
  ArrowUp: WasmArm.North,
  ArrowRight: WasmArm.East,
  ArrowDown: WasmArm.South,
  ArrowLeft: WasmArm.West,
}

export function useKeyboardInput(gameRef: RefObject<WasmCrossGame | null>, enabled: boolean) {
  useEffect(() => {
    if (!enabled) return

    const handleKeyDown = (event: KeyboardEvent) => {
      const game = gameRef.current
      if (!game || event.repeat) return

      if (game.awaiting_well_selection()) {
        const arm = DIGIT_WELL_KEYS[event.code] ?? ARROW_WELL_KEYS[event.code]
        if (arm !== undefined) {
          event.preventDefault()
          game.select_well(arm)
        }
        return
      }

      const action = MOVE_ACTIONS[event.code]
      if (action) {
        event.preventDefault()
        action(game)
      }
    }

    const handleKeyUp = (event: KeyboardEvent) => {
      const game = gameRef.current
      if (!game) return
      if (event.code === 'ArrowDown') {
        game.soft_drop_end()
      }
    }

    window.addEventListener('keydown', handleKeyDown)
    window.addEventListener('keyup', handleKeyUp)
    return () => {
      window.removeEventListener('keydown', handleKeyDown)
      window.removeEventListener('keyup', handleKeyUp)
    }
  }, [gameRef, enabled])
}
