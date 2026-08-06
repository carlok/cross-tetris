import { forwardRef, useImperativeHandle, useRef } from 'react'

export interface SelectionTimerHandle {
  /** 1 = full time remaining, 0 = about to auto-select. Updated every frame
   * directly on the DOM (not via React state) so it animates smoothly
   * without forcing a rerender every tick. */
  setFraction(remaining: number): void
}

export interface SelectionTimerProps {
  width?: number
}

export const SelectionTimer = forwardRef<SelectionTimerHandle, SelectionTimerProps>(function SelectionTimer(
  { width = 48 },
  ref,
) {
  const barRef = useRef<HTMLDivElement>(null)

  useImperativeHandle(ref, () => ({
    setFraction(remaining: number) {
      const bar = barRef.current
      if (!bar) return
      const clamped = Math.max(0, Math.min(1, remaining))
      bar.style.width = `${clamped * 100}%`
      bar.style.background = clamped < 0.25 ? '#f44336' : '#4dd0e1'
    },
  }))

  return (
    <div style={{ width, height: 4, background: '#333', borderRadius: 2, overflow: 'hidden' }}>
      <div ref={barRef} style={{ height: '100%', width: '100%', background: '#4dd0e1' }} />
    </div>
  )
})
