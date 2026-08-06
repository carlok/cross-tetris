// Vibration helper, shared by the gamepad input hook and by App.tsx's
// state-diffed effects (lock/line-clear/game-over). Looks up the gamepad
// fresh each call rather than holding a reference — Gamepad objects from
// the Gamepad API are snapshots, not live references.

export function getPrimaryGamepad(): Gamepad | null {
  if (typeof navigator === 'undefined' || !navigator.getGamepads) return null
  for (const pad of navigator.getGamepads()) {
    if (pad) return pad
  }
  return null
}

interface GamepadHaptics {
  vibrationActuator?: {
    playEffect?: (type: string, params: { duration: number; strongMagnitude: number; weakMagnitude: number }) => Promise<unknown>
  }
  hapticActuators?: Array<{ pulse?: (intensity: number, durationMs: number) => Promise<unknown> }>
}

/** Best-effort — the Gamepad haptics API is still experimental and varies
 * by browser; silently does nothing if unsupported. */
export function vibrateGamepad(durationMs: number, intensity = 0.6) {
  const pad = getPrimaryGamepad()
  if (!pad) return
  const haptics = pad as unknown as GamepadHaptics
  try {
    if (haptics.vibrationActuator?.playEffect) {
      haptics.vibrationActuator.playEffect('dual-rumble', {
        duration: durationMs,
        strongMagnitude: intensity,
        weakMagnitude: intensity,
      })
      return
    }
    haptics.hapticActuators?.[0]?.pulse?.(intensity, durationMs)
  } catch {
    // unsupported or not yet allowed (needs a user gesture) — ignore
  }
}
