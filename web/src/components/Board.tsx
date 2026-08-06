import { forwardRef, useImperativeHandle, useRef } from 'react'

export const DEFAULT_CELL_SIZE = 24
export const BOARD_WIDTH = 10
export const BOARD_HEIGHT = 20

// index 0 = empty; 1..=7 = I, J, L, O, S, Z, T (matches engine::PieceKind::as_u8)
const COLORS = ['#4dd0e1', '#3f51b5', '#ff9800', '#ffeb3b', '#4caf50', '#f44336', '#9c27b0']

/**
 * Which screen direction increasing engine row (gravity's axis) maps to.
 * The engine's board model is always the same 10-wide x 20-tall grid with
 * gravity pulling toward row 19, regardless of which cross arm it's in
 * (the "canonical internal orientation" the project spec calls for) — this
 * is purely a rendering transform, so pieces visually spawn near the
 * center-facing edge of their well and fall outward, away from the center:
 * North falls upward (entry at the bottom, near the center), South falls
 * downward (entry at the top), East falls rightward (entry at the left,
 * landscape), West falls leftward (entry at the right, landscape).
 */
export type GravityDirection = 'down' | 'up' | 'left' | 'right'

export interface BoardHandle {
  draw(buffer: Uint8Array): void
}

export interface BoardProps {
  cellSize?: number
  gravityDirection?: GravityDirection
}

export const Board = forwardRef<BoardHandle, BoardProps>(function Board(
  { cellSize = DEFAULT_CELL_SIZE, gravityDirection = 'down' },
  ref,
) {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const landscape = gravityDirection === 'left' || gravityDirection === 'right'
  const canvasWidth = (landscape ? BOARD_HEIGHT : BOARD_WIDTH) * cellSize
  const canvasHeight = (landscape ? BOARD_WIDTH : BOARD_HEIGHT) * cellSize

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
          // Each case is a proper rotation of the 'down' (South, identity)
          // mapping — never a plain axis swap/single-axis flip, which would
          // mirror the piece (e.g. S becomes Z) instead of just reorienting
          // it. 'up' is a 180° rotation (both axes flip); 'left'/'right' are
          // 90° CW/CCW rotations of the whole board image.
          let x: number
          let y: number
          switch (gravityDirection) {
            case 'down':
              x = col * cellSize
              y = row * cellSize
              break
            case 'up':
              x = (BOARD_WIDTH - 1 - col) * cellSize
              y = (BOARD_HEIGHT - 1 - row) * cellSize
              break
            case 'right': // 90° CCW: South's bottom edge swings to the right
              x = row * cellSize
              y = (BOARD_WIDTH - 1 - col) * cellSize
              break
            case 'left': // 90° CW: South's bottom edge swings to the left
              x = (BOARD_HEIGHT - 1 - row) * cellSize
              y = col * cellSize
              break
          }
          ctx.fillStyle = COLORS[value - 1] ?? '#999'
          ctx.fillRect(x + 1, y + 1, cellSize - 2, cellSize - 2)
        }
      }
    },
  }))

  return (
    <canvas
      ref={canvasRef}
      width={canvasWidth}
      height={canvasHeight}
      style={{ background: '#000', border: '2px solid #444', imageRendering: 'pixelated' }}
    />
  )
})
