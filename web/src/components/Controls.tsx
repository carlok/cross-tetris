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
        <div>Arrows: move / rotate</div>
        <div>Down: soft drop</div>
        <div>Space: hard drop</div>
        <div>C: hold</div>
      </div>
    </div>
  )
}
