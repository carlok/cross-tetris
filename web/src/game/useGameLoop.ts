import { useEffect, useRef, type RefObject } from 'react'
import type { WasmCrossGame } from '../wasm'

const AI_STEP_INTERVAL_MS = 500

/**
 * Drives the 4-arm cross game via requestAnimationFrame: ticks gravity/
 * lock-delay on every arm every frame, optionally invokes the greedy AI on
 * all non-topped-out arms on a fixed interval (so it doesn't play instant-
 * perfect every frame), and calls `onFrame` afterward so the caller can
 * repaint each arm's board and sync React state without a rerender per frame.
 */
export function useGameLoop(gameRef: RefObject<WasmCrossGame | null>, aiEnabled: boolean, onFrame: () => void) {
  const aiEnabledRef = useRef(aiEnabled)
  aiEnabledRef.current = aiEnabled
  const onFrameRef = useRef(onFrame)
  onFrameRef.current = onFrame

  useEffect(() => {
    let raf = 0
    let last = performance.now()
    let aiAccumMs = 0

    const step = (now: number) => {
      const dt = now - last
      last = now
      const game = gameRef.current
      if (game && !game.is_game_over()) {
        game.tick(dt)
        if (aiEnabledRef.current) {
          aiAccumMs += dt
          if (aiAccumMs >= AI_STEP_INTERVAL_MS) {
            aiAccumMs = 0
            game.ai_step_all()
          }
        } else {
          aiAccumMs = 0
        }
      }
      onFrameRef.current()
      raf = requestAnimationFrame(step)
    }

    raf = requestAnimationFrame(step)
    return () => cancelAnimationFrame(raf)
  }, [gameRef])
}
