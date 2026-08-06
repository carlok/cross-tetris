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

// Number keys 1-4 commit the next queued piece to a well. Only takes effect
// while awaiting selection (the engine ignores it otherwise).
const WELL_SELECT_KEYS: Record<string, WasmArm> = {
  Digit1: WasmArm.North,
  Digit2: WasmArm.East,
  Digit3: WasmArm.South,
  Digit4: WasmArm.West,
}

export function useKeyboardInput(gameRef: RefObject<WasmCrossGame | null>, enabled: boolean) {
  useEffect(() => {
    if (!enabled) return

    const handleKeyDown = (event: KeyboardEvent) => {
      const game = gameRef.current
      if (!game || event.repeat) return

      if (event.code in WELL_SELECT_KEYS) {
        event.preventDefault()
        game.select_well(WELL_SELECT_KEYS[event.code])
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
