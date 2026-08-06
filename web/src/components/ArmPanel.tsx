import { forwardRef } from 'react'
import { Board, type BoardHandle, type GravityDirection } from './Board'

export type ArmHighlight = 'active' | 'selectable' | 'blocked' | 'none'

export interface ArmPanelProps {
  label: string
  score: number
  gameOver: boolean
  highlight: ArmHighlight
  cellSize?: number
  gravityDirection: GravityDirection
}

const BORDER: Record<ArmHighlight, string> = {
  active: '2px solid #4dd0e1',
  selectable: '2px dashed #666',
  blocked: '2px dashed #b71c1c',
  none: '2px solid transparent',
}

const OPACITY: Record<ArmHighlight, number> = {
  active: 1,
  selectable: 1,
  blocked: 0.55,
  none: 1,
}

export const ArmPanel = forwardRef<BoardHandle, ArmPanelProps>(function ArmPanel(
  { label, score, gameOver, highlight, cellSize, gravityDirection },
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
        opacity: OPACITY[highlight],
      }}
    >
      <div style={{ fontFamily: 'monospace', fontSize: 12, opacity: 0.85 }}>
        {label} {score}
        {gameOver ? ' — OVER' : highlight === 'blocked' ? ' (wait)' : ''}
      </div>
      <Board ref={ref} cellSize={cellSize} gravityDirection={gravityDirection} />
    </div>
  )
})
