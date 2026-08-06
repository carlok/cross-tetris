export interface HudProps {
  totalScore: number
  gameOver: boolean
}

export function Hud({ totalScore, gameOver }: HudProps) {
  return (
    <div style={{ fontFamily: 'monospace', minWidth: 140 }}>
      <div style={{ fontSize: 18 }}>Total: {totalScore}</div>
      {gameOver && <div style={{ color: '#f44336', fontWeight: 'bold', marginTop: 8 }}>GAME OVER (an arm topped out)</div>}
    </div>
  )
}
