import { useEffect, type RefObject } from 'react'
import { WasmArm, type WasmCrossGame } from '../wasm'
import { effects } from './effects'
import { resolveDirection, type ArrowKey, type Semantic } from './controls'

// No DAS/ARR (auto-repeat) tuning in this milestone (documented follow-up).
// Directional keys are resolved through controls.ts against whichever arm
// is currently falling, so they stay screen-relative regardless of the
// well's rendering orientation. Rotate-CCW (Z), hard drop (Space), and hold
// (C) aren't directional, so they stay fixed regardless of arm.
// Hard drop's lock feedback comes from the state-diffed `effects.lock()` in
// App.tsx (covers hard drop, natural lock, and AI/auto placement uniformly
// without double-beeping), so it's not triggered directly here.
const ARROW_KEYS: Record<string, ArrowKey> = {
  ArrowUp: 'up',
  ArrowDown: 'down',
  ArrowLeft: 'left',
  ArrowRight: 'right',
}

function applySemantic(game: WasmCrossGame, semantic: Semantic) {
  switch (semantic) {
    case 'moveLeft':
      game.move_left()
      effects.move()
      break
    case 'moveRight':
      game.move_right()
      effects.move()
      break
    case 'softDrop':
      game.soft_drop_start()
      effects.softDrop()
      break
    case 'rotateCw':
      game.rotate_cw()
      effects.rotate()
      break
  }
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
          // Sound for a successful selection comes from App.tsx's state
          // diff (covers manual, auto-timeout, and stays a single call site).
          game.select_well(arm)
        }
        return
      }

      const arrow = ARROW_KEYS[event.code]
      if (arrow) {
        event.preventDefault()
        const arm = game.active_arm()
        if (arm >= 0) applySemantic(game, resolveDirection(arm as WasmArm, arrow))
        return
      }

      if (event.code === 'KeyZ') {
        event.preventDefault()
        game.rotate_ccw()
        effects.rotate()
      } else if (event.code === 'Space') {
        event.preventDefault()
        game.hard_drop()
      } else if (event.code === 'KeyC') {
        event.preventDefault()
        game.hold()
        effects.hold()
      }
    }

    const handleKeyUp = (event: KeyboardEvent) => {
      const game = gameRef.current
      if (!game) return
      const arrow = ARROW_KEYS[event.code]
      if (!arrow) return
      const arm = game.active_arm()
      if (arm >= 0 && resolveDirection(arm as WasmArm, arrow) === 'softDrop') {
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
