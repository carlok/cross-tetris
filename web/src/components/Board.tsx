import { forwardRef, useImperativeHandle, useRef } from 'react'

export const CELL_SIZE = 24
export const BOARD_WIDTH = 10
export const BOARD_HEIGHT = 20

// index 0 = empty; 1..=7 = I, J, L, O, S, Z, T (matches engine::PieceKind::as_u8)
const COLORS = ['#4dd0e1', '#3f51b5', '#ff9800', '#ffeb3b', '#4caf50', '#f44336', '#9c27b0']

export interface BoardHandle {
  draw(buffer: Uint8Array): void
}

export const Board = forwardRef<BoardHandle>(function Board(_props, ref) {
  const canvasRef = useRef<HTMLCanvasElement>(null)

  useImperativeHandle(ref, () => ({
    draw(buffer: Uint8Array) {
      const canvas = canvasRef.current
      if (!canvas) return
      const ctx = canvas.getContext('2d')
      if (!ctx) return
      ctx.fillStyle = '#000'
      ctx.fillRect(0, 0, canvas.width, canvas.height)
      for (let row = 0; row < BOARD_HEIGHT; row++) {
        for (let col = 0; col < BOARD_WIDTH; col++) {
          const value = buffer[row * BOARD_WIDTH + col]
          if (!value) continue
          ctx.fillStyle = COLORS[value - 1] ?? '#999'
          ctx.fillRect(col * CELL_SIZE + 1, row * CELL_SIZE + 1, CELL_SIZE - 2, CELL_SIZE - 2)
        }
      }
    },
  }))

  return (
    <canvas
      ref={canvasRef}
      width={BOARD_WIDTH * CELL_SIZE}
      height={BOARD_HEIGHT * CELL_SIZE}
      style={{ background: '#000', border: '2px solid #444', imageRendering: 'pixelated' }}
    />
  )
})
