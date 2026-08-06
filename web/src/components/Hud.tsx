const PIECE_NAMES = ['', 'I', 'J', 'L', 'O', 'S', 'Z', 'T']

export interface HudProps {
  totalScore: number
  gameOver: boolean
  nextPieceKind: number // 0 = none
  awaitingSelection: boolean
}

export function Hud({ totalScore, gameOver, nextPieceKind, awaitingSelection }: HudProps) {
  return (
    <div style={{ fontFamily: 'monospace', minWidth: 140 }}>
      <div style={{ fontSize: 18 }}>Total: {totalScore}</div>
      <div style={{ marginTop: 8 }}>Next piece: {nextPieceKind > 0 ? PIECE_NAMES[nextPieceKind] : '-'}</div>
      {awaitingSelection && !gameOver && (
        <div style={{ color: '#4dd0e1', marginTop: 8 }}>Pick a well (1-4) for this piece</div>
      )}
      {gameOver && <div style={{ color: '#f44336', fontWeight: 'bold', marginTop: 8 }}>GAME OVER (a well topped out)</div>}
    </div>
  )
}
