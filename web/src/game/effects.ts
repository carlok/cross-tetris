// Central place input hooks and App.tsx call into for feedback, so "what
// happened" (an input handler, a state diff in the game loop) stays decoupled
// from "how to react" (sound always; vibration additionally when a gamepad
// is connected and supports it).

import { sound } from './sound'
import { vibrateGamepad } from './gamepad'

export const effects = {
  move: () => sound.move(),
  rotate: () => sound.rotate(),
  softDrop: () => sound.softDrop(),
  hold: () => sound.hold(),
  select: () => sound.select(),
  lock: () => {
    sound.lock()
    vibrateGamepad(50, 0.35)
  },
  lineClear: (linesCleared: number) => {
    sound.lineClear(linesCleared)
    vibrateGamepad(150, 0.8)
  },
  gameOver: () => {
    sound.gameOver()
    vibrateGamepad(400, 1.0)
  },
}
