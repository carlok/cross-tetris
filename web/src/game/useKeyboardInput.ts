import { useEffect, type RefObject } from 'react'
import type { WasmGame } from '../wasm'

// Direct 1:1 keydown -> action mapping, no DAS/ARR (auto-repeat) tuning in
// this milestone (documented follow-up).
const KEYDOWN_ACTIONS: Record<string, (g: WasmGame) => void> = {
  ArrowLeft: (g) => g.move_left(),
  ArrowRight: (g) => g.move_right(),
  ArrowUp: (g) => g.rotate_cw(),
  KeyZ: (g) => g.rotate_ccw(),
  ArrowDown: (g) => g.soft_drop_start(),
  Space: (g) => g.hard_drop(),
  KeyC: (g) => g.hold(),
}

export function useKeyboardInput(gameRef: RefObject<WasmGame | null>, enabled: boolean) {
  useEffect(() => {
    if (!enabled) return

    const handleKeyDown = (event: KeyboardEvent) => {
      const game = gameRef.current
      if (!game || event.repeat) return
      const action = KEYDOWN_ACTIONS[event.code]
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
