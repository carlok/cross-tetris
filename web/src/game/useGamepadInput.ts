import { useEffect, useRef, type RefObject } from 'react'
import { WasmArm, type WasmCrossGame } from '../wasm'
import { effects } from './effects'
import { getPrimaryGamepad } from './gamepad'
import { resolveDirection, type ArrowKey, type Semantic } from './controls'

// Standard gamepad mapping (W3C "standard" layout — Xbox/PS-style pads).
const BUTTON = {
  A: 0, // bottom face button
  B: 1, // right face button
  X: 2, // left face button
  Y: 3, // top face button
  DPAD_UP: 12,
  DPAD_DOWN: 13,
  DPAD_LEFT: 14,
  DPAD_RIGHT: 15,
} as const

const STICK_DEADZONE = 0.5
const POLL_INTERVAL_MS = 50 // 20Hz is plenty for discrete button edges

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

/**
 * Polls the Gamepad API (no event-based API exists for button presses) and
 * mirrors `useKeyboardInput`: D-pad/left-stick double as well selection
 * while awaiting one (Up=N, Right=E, Down=S, Left=W, spatially matching the
 * cross layout) and as screen-relative movement once a piece is falling,
 * resolved per-arm through controls.ts — same non-conflict reasoning as the
 * keyboard arrows, since selection and movement never apply at the same
 * time. Y/B are fixed rotate CW/CCW buttons (not directional, so no
 * per-arm resolution needed — same as the keyboard's Z key).
 */
export function useGamepadInput(gameRef: RefObject<WasmCrossGame | null>, enabled: boolean) {
  const wasPressed = useRef<Record<string, boolean>>({})
  const softDropHeld = useRef(false)

  useEffect(() => {
    if (!enabled) return

    const interval = setInterval(() => {
      const game = gameRef.current
      const pad = getPrimaryGamepad()
      if (!game || !pad) return

      const pressed = (i: number) => pad.buttons[i]?.pressed ?? false
      const axisX = pad.axes[0] ?? 0
      const axisY = pad.axes[1] ?? 0
      const up = pressed(BUTTON.DPAD_UP) || axisY < -STICK_DEADZONE
      const down = pressed(BUTTON.DPAD_DOWN) || axisY > STICK_DEADZONE
      const left = pressed(BUTTON.DPAD_LEFT) || axisX < -STICK_DEADZONE
      const right = pressed(BUTTON.DPAD_RIGHT) || axisX > STICK_DEADZONE

      const prev = wasPressed.current
      const onPress = (key: string, isDown: boolean, action: () => void) => {
        if (isDown && !prev[key]) action()
        prev[key] = isDown
      }

      if (game.awaiting_well_selection()) {
        // Sound for a successful selection comes from App.tsx's state diff
        // (covers manual, auto-timeout, and stays a single call site).
        onPress('up', up, () => game.select_well(WasmArm.North))
        onPress('right', right, () => game.select_well(WasmArm.East))
        onPress('down', down, () => game.select_well(WasmArm.South))
        onPress('left', left, () => game.select_well(WasmArm.West))
      } else {
        const arm = game.active_arm()
        const directions: [ArrowKey, boolean][] = [
          ['up', up],
          ['down', down],
          ['left', left],
          ['right', right],
        ]
        let softDropDown = false
        for (const [dir, isDown] of directions) {
          const semantic = arm >= 0 ? resolveDirection(arm as WasmArm, dir) : null
          if (semantic === 'softDrop' && isDown) softDropDown = true
          onPress(dir, isDown, () => {
            if (semantic) applySemantic(game, semantic)
          })
        }

        onPress('Y', pressed(BUTTON.Y), () => {
          game.rotate_cw()
          effects.rotate()
        })
        onPress('B', pressed(BUTTON.B), () => {
          game.rotate_ccw()
          effects.rotate()
        })
        onPress('X', pressed(BUTTON.X), () => {
          game.hold()
          effects.hold()
        })
        onPress('A', pressed(BUTTON.A), () => {
          game.rotate_cw()
          effects.rotate()
          game.hard_drop()
        })

        if (softDropDown && !softDropHeld.current) {
          softDropHeld.current = true
        } else if (!softDropDown && softDropHeld.current) {
          game.soft_drop_end()
          softDropHeld.current = false
        }
      }
    }, POLL_INTERVAL_MS)

    return () => clearInterval(interval)
  }, [gameRef, enabled])
}
