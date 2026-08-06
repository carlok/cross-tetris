// index 0 = empty; 1..=7 = I, J, L, O, S, Z, T (matches engine::PieceKind::as_u8)
const COLORS = ['#4dd0e1', '#3f51b5', '#ff9800', '#ffeb3b', '#4caf50', '#f44336', '#9c27b0']

// R0 (spawn) shapes as (row, col) offsets in a 4x4 box, matching
// engine::rotation::shape(kind, Rotation::R0) for I/J/L/S/Z/T; O is shifted
// to rows 1-2 so it sits visually centered in the same 4x4 preview box.
const SHAPES: [number, number][][] = [
  [[1, 0], [1, 1], [1, 2], [1, 3]], // I
  [[0, 0], [1, 0], [1, 1], [1, 2]], // J
  [[0, 2], [1, 0], [1, 1], [1, 2]], // L
  [[1, 1], [1, 2], [2, 1], [2, 2]], // O
  [[0, 1], [0, 2], [1, 0], [1, 1]], // S
  [[0, 0], [0, 1], [1, 1], [1, 2]], // Z
  [[0, 1], [1, 0], [1, 1], [1, 2]], // T
]

export interface PiecePreviewProps {
  kind: number // 0 = none, 1..=7 = PieceKind
  cellSize?: number
}

export function PiecePreview({ kind, cellSize = 14 }: PiecePreviewProps) {
  const size = 4 * cellSize
  const shape = kind > 0 ? SHAPES[kind - 1] : null
  return (
    <svg width={size} height={size} style={{ display: 'block' }}>
      {shape?.map(([row, col], i) => (
        <rect
          key={i}
          x={col * cellSize + 1}
          y={row * cellSize + 1}
          width={cellSize - 2}
          height={cellSize - 2}
          fill={COLORS[kind - 1]}
        />
      ))}
    </svg>
  )
}
