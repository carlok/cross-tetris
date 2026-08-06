import { useEffect, type RefObject } from 'react'
import { WasmArm, type WasmCrossGame } from '../wasm'

// Direct 1:1 keydown -> action mapping, no DAS/ARR (auto-repeat) tuning in
// this milestone (documented follow-up). Actions apply to whichever arm is
// currently selected.
const MOVE_ACTIONS: Record<string, (g: WasmCrossGame, arm: WasmArm) => void> = {
  ArrowLeft: (g, arm) => g.move_left(arm),
  ArrowRight: (g, arm) => g.move_right(arm),
  ArrowUp: (g, arm) => g.rotate_cw(arm),
  KeyZ: (g, arm) => g.rotate_ccw(arm),
  ArrowDown: (g, arm) => g.soft_drop_start(arm),
  Space: (g, arm) => g.hard_drop(arm),
  KeyC: (g, arm) => g.hold(arm),
}

// Number keys 1-4 switch which arm receives movement input — the human
// equivalent of the AI being able to "look at" a different board.
const ARM_SELECT_KEYS: Record<string, WasmArm> = {
  Digit1: WasmArm.North,
  Digit2: WasmArm.East,
  Digit3: WasmArm.South,
  Digit4: WasmArm.West,
}

export function useKeyboardInput(
  gameRef: RefObject<WasmCrossGame | null>,
  selectedArmRef: RefObject<WasmArm>,
  onSelectArm: (arm: WasmArm) => void,
  enabled: boolean,
) {
  useEffect(() => {
    if (!enabled) return

    const handleKeyDown = (event: KeyboardEvent) => {
      const game = gameRef.current
      if (!game || event.repeat) return

      if (event.code in ARM_SELECT_KEYS) {
        event.preventDefault()
        onSelectArm(ARM_SELECT_KEYS[event.code])
        return
      }

      const action = MOVE_ACTIONS[event.code]
      if (action) {
        event.preventDefault()
        action(game, selectedArmRef.current)
      }
    }

    const handleKeyUp = (event: KeyboardEvent) => {
      const game = gameRef.current
      if (!game) return
      if (event.code === 'ArrowDown') {
        game.soft_drop_end(selectedArmRef.current)
      }
    }

    window.addEventListener('keydown', handleKeyDown)
    window.addEventListener('keyup', handleKeyUp)
    return () => {
      window.removeEventListener('keydown', handleKeyDown)
      window.removeEventListener('keyup', handleKeyUp)
    }
  }, [gameRef, selectedArmRef, onSelectArm, enabled])
}
