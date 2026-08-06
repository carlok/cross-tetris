export interface ControlsProps {
  aiEnabled: boolean
  onToggleAi: () => void
  onRestart: () => void
}

export function Controls({ aiEnabled, onToggleAi, onRestart }: ControlsProps) {
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
      <button onClick={onToggleAi}>{aiEnabled ? 'Switch to Human' : 'Switch to AI'}</button>
      <button onClick={onRestart}>Restart</button>
      <div style={{ fontFamily: 'monospace', fontSize: 12, opacity: 0.7 }}>
        <div>1-4: send next piece to N/E/S/W</div>
        <div>Arrows: move / rotate</div>
        <div>Down: soft drop</div>
        <div>Space: hard drop</div>
        <div>C: hold</div>
        <div style={{ marginTop: 6, opacity: 0.6 }}>
          One shared piece queue. You pick which well each piece goes to,
          then play it out normally in that well.
          <br />
          AI mode: the AI evaluates all four wells for each piece and routes
          it to the best one.
        </div>
      </div>
    </div>
  )
}
