import { forwardRef } from 'react'
import { Board, type BoardHandle } from './Board'

export type ArmHighlight = 'active' | 'selectable' | 'none'

export interface ArmPanelProps {
  label: string
  score: number
  gameOver: boolean
  highlight: ArmHighlight
  cellSize?: number
}

const BORDER: Record<ArmHighlight, string> = {
  active: '2px solid #4dd0e1',
  selectable: '2px dashed #666',
  none: '2px solid transparent',
}

export const ArmPanel = forwardRef<BoardHandle, ArmPanelProps>(function ArmPanel(
  { label, score, gameOver, highlight, cellSize },
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
        border: BORDER[highlight],
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
