const PIECE_NAMES = ['', 'I', 'J', 'L', 'O', 'S', 'Z', 'T']

export interface HudProps {
  score: number
  level: number
  linesCleared: number
  gameOver: boolean
  holdKind: number // -1 = empty, 1..=7 = PieceKind
  nextQueue: number[]
}

export function Hud({ score, level, linesCleared, gameOver, holdKind, nextQueue }: HudProps) {
  return (
    <div style={{ fontFamily: 'monospace', minWidth: 140 }}>
      <div>Score: {score}</div>
      <div>Level: {level}</div>
      <div>Lines: {linesCleared}</div>
      <div>Hold: {holdKind >= 0 ? PIECE_NAMES[holdKind] : '-'}</div>
      <div>Next: {nextQueue.map((k) => PIECE_NAMES[k]).join(' ')}</div>
      {gameOver && <div style={{ color: '#f44336', fontWeight: 'bold', marginTop: 8 }}>GAME OVER</div>}
    </div>
  )
}
