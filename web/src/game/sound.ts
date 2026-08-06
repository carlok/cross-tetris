// Synthesized beeps via Web Audio — no external asset files, so the game
// stays self-contained. AudioContext must start after a user gesture per
// browser autoplay policy; `getCtx` creates/resumes it lazily on first use
// rather than at module load, so it's safe to import this eagerly.

let ctx: AudioContext | null = null

function getCtx(): AudioContext | null {
  if (typeof window === 'undefined') return null
  if (!ctx) {
    const Ctor = window.AudioContext ?? (window as unknown as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext
    if (!Ctor) return null
    ctx = new Ctor()
  }
  if (ctx.state === 'suspended') {
    ctx.resume().catch(() => {})
  }
  return ctx
}

function beep(freq: number, durationMs: number, type: OscillatorType = 'square', volume = 0.05) {
  const audio = getCtx()
  if (!audio) return
  const osc = audio.createOscillator()
  const gain = audio.createGain()
  osc.type = type
  osc.frequency.value = freq
  osc.connect(gain)
  gain.connect(audio.destination)
  const now = audio.currentTime
  gain.gain.setValueAtTime(volume, now)
  gain.gain.exponentialRampToValueAtTime(0.0001, now + durationMs / 1000)
  osc.start(now)
  osc.stop(now + durationMs / 1000)
}

// A short two-note blip, used for line clears (bigger clears get an extra note).
function chime(baseFreq: number, notes: number, durationMs: number, volume: number) {
  const audio = getCtx()
  if (!audio) return
  for (let i = 0; i < notes; i++) {
    const start = audio.currentTime + i * (durationMs / 1000) * 0.6
    const osc = audio.createOscillator()
    const gain = audio.createGain()
    osc.type = 'sine'
    osc.frequency.value = baseFreq * Math.pow(1.25, i)
    osc.connect(gain)
    gain.connect(audio.destination)
    gain.gain.setValueAtTime(volume, start)
    gain.gain.exponentialRampToValueAtTime(0.0001, start + durationMs / 1000)
    osc.start(start)
    osc.stop(start + durationMs / 1000)
  }
}

export const sound = {
  move: () => beep(220, 25, 'square', 0.025),
  rotate: () => beep(330, 35, 'square', 0.035),
  softDrop: () => beep(150, 20, 'square', 0.02),
  hold: () => beep(440, 50, 'triangle', 0.05),
  select: () => beep(523, 60, 'sine', 0.06),
  /** A piece just locked (hard drop, natural lock, or AI/auto placement). */
  lock: () => beep(90, 70, 'sawtooth', 0.06),
  lineClear: (linesCleared: number) => chime(660, Math.min(linesCleared, 4), 90, 0.07),
  gameOver: () => beep(110, 500, 'sawtooth', 0.09),
}
