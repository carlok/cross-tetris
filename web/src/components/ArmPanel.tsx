import { forwardRef } from 'react'
import { Board, type BoardHandle } from './Board'

export interface ArmPanelProps {
  label: string
  score: number
  gameOver: boolean
  selected: boolean
  cellSize?: number
}

export const ArmPanel = forwardRef<BoardHandle, ArmPanelProps>(function ArmPanel(
  { label, score, gameOver, selected, cellSize },
  ref,
) {
  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        gap: 4,
        padding: 4,
        border: selected ? '2px solid #4dd0e1' : '2px solid transparent',
        borderRadius: 4,
      }}
    >
      <div style={{ fontFamily: 'monospace', fontSize: 12, opacity: 0.85 }}>
        {label} {score}
        {gameOver ? ' — OVER' : ''}
      </div>
      <Board ref={ref} cellSize={cellSize} />
    </div>
  )
})
