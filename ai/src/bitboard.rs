//! Colorless, allocation-free board used only inside the AI's search.
//!
//! `engine::Board` stores `Option<PieceKind>` per cell because rendering
//! needs the piece colors, and every engine test depends on its exact API —
//! it is not touched here. `BitBoard` is a search-only mirror: one `u16` per
//! row (bits 0..=9 = columns 0..=9), `Copy`, 80 bytes. "Cloning" it per
//! candidate placement is a stack copy, not a heap allocation, unlike
//! `engine::Board::clone()` (a `Vec<Option<PieceKind>>` clone) which the AI
//! previously did once per candidate — roughly 40 times per well per
//! decision.

use engine::board::{Board, BOARD_TOTAL_HEIGHT, BOARD_WIDTH};
use engine::piece::ActivePiece;

const FULL_ROW_MASK: u16 = (1 << BOARD_WIDTH) - 1;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct BitBoard {
    /// rows[0] = topmost (hidden) row, rows[BOARD_TOTAL_HEIGHT - 1] = bottom
    /// visible row — same row convention as `engine::Board`.
    rows: [u16; BOARD_TOTAL_HEIGHT],
}

impl BitBoard {
    pub fn empty() -> Self {
        BitBoard { rows: [0; BOARD_TOTAL_HEIGHT] }
    }

    /// One-time conversion from the engine's colored board. Called once per
    /// well per AI decision, not once per candidate placement — the board
    /// doesn't change across the ~40 candidates a single decision considers.
    pub fn from_board(board: &Board) -> Self {
        let mut rows = [0u16; BOARD_TOTAL_HEIGHT];
        for (r, row_mask) in rows.iter_mut().enumerate() {
            let mut mask = 0u16;
            for c in 0..BOARD_WIDTH {
                if board.is_occupied(r as i32, c as i32) {
                    mask |= 1 << c;
                }
            }
            *row_mask = mask;
        }
        BitBoard { rows }
    }

    pub fn is_row_full(&self, row: usize) -> bool {
        self.rows[row] == FULL_ROW_MASK
    }

    pub fn is_row_empty(&self, row: usize) -> bool {
        self.rows[row] == 0
    }

    /// Same semantics as `engine::Board::get`/`is_occupied`: out-of-bounds
    /// columns/floor read as occupied (walls), out-of-bounds rows above the
    /// top read as open (unoccupied) — pieces spawn and fall through the
    /// hidden rows above row 0 freely.
    pub fn is_occupied(&self, row: i32, col: i32) -> bool {
        if row < 0 {
            return false;
        }
        if row as usize >= BOARD_TOTAL_HEIGHT || col < 0 || col as usize >= BOARD_WIDTH {
            return true;
        }
        (self.rows[row as usize] >> col) & 1 != 0
    }

    pub fn set(&mut self, row: i32, col: i32) {
        if row >= 0 && (row as usize) < BOARD_TOTAL_HEIGHT && col >= 0 && (col as usize) < BOARD_WIDTH {
            self.rows[row as usize] |= 1 << col;
        }
    }

    /// Clears every full row, collapsing the rows above down to fill the
    /// gap. Returns the number of rows cleared. Mirrors
    /// `engine::Board::clear_full_rows` exactly (same top-to-bottom
    /// collapse), verified by a differential test against it.
    pub fn clear_full_rows(&mut self) -> u32 {
        let mut write = BOARD_TOTAL_HEIGHT as i32 - 1;
        let mut cleared = 0u32;
        for read in (0..BOARD_TOTAL_HEIGHT as i32).rev() {
            if self.rows[read as usize] == FULL_ROW_MASK {
                cleared += 1;
                continue;
            }
            if write != read {
                self.rows[write as usize] = self.rows[read as usize];
            }
            write -= 1;
        }
        while write >= 0 {
            self.rows[write as usize] = 0;
            write -= 1;
        }
        cleared
    }

    /// Same definition as `engine::Board::column_height`: `BOARD_TOTAL_HEIGHT
    /// - (row of the topmost filled cell)`, or `0` for an empty column.
    pub fn column_height(&self, col: usize) -> u32 {
        for (r, row_mask) in self.rows.iter().enumerate() {
            if (row_mask >> col) & 1 != 0 {
                return (BOARD_TOTAL_HEIGHT - r) as u32;
            }
        }
        0
    }

    pub fn place(&mut self, piece: &ActivePiece) {
        for (row, col) in piece.occupied_cells() {
            self.set(row, col);
        }
    }

    /// Row index of the topmost non-empty row, or `BOARD_TOTAL_HEIGHT` if
    /// the board is entirely empty. Shared by every feature that must not
    /// count the (many) entirely-empty hidden rows above the actual stack —
    /// counting them would bias the feature toward taller stacks, since a
    /// shorter stack simply has more empty rows to (wrongly) count.
    pub fn topmost_filled_row(&self) -> usize {
        for (r, row_mask) in self.rows.iter().enumerate() {
            if *row_mask != 0 {
                return r;
            }
        }
        BOARD_TOTAL_HEIGHT
    }

    /// Empty cells with at least one filled cell above them in the same
    /// column.
    pub fn count_holes(&self) -> u32 {
        let mut holes = 0;
        for col in 0..BOARD_WIDTH {
            let mut found_filled = false;
            for row_mask in self.rows.iter() {
                let filled = (row_mask >> col) & 1 != 0;
                if filled {
                    found_filled = true;
                } else if found_filled {
                    holes += 1;
                }
            }
        }
        holes
    }
}

impl Default for BitBoard {
    fn default() -> Self {
        Self::empty()
    }
}

pub fn piece_fits(board: &BitBoard, piece: &ActivePiece) -> bool {
    piece.occupied_cells().iter().all(|&(row, col)| !board.is_occupied(row, col))
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::piece::PieceKind;
    use engine::rng::Rng;

    /// For many boards (including hand-built edge cases and pseudo-random
    /// stacks), BitBoard must agree with engine::Board on column_height for
    /// every column, is_row_full for every row, and clear_full_rows must
    /// return the same count and leave an equivalent resulting board — this
    /// is what proves the bitboard is a faithful mirror, not just "compiles
    /// and looks right".
    fn engine_boards() -> Vec<Board> {
        let mut boards = vec![Board::new()];

        let mut flat = Board::new();
        for col in 0..BOARD_WIDTH as i32 {
            for row in 35..40 {
                flat.set(row, col, Some(PieceKind::J));
            }
        }
        boards.push(flat);

        let mut one_full_row = Board::new();
        for col in 0..BOARD_WIDTH as i32 {
            one_full_row.set(39, col, Some(PieceKind::L));
        }
        for col in 0..BOARD_WIDTH as i32 {
            if col != 3 {
                one_full_row.set(38, col, Some(PieceKind::T));
            }
        }
        boards.push(one_full_row);

        let mut multiple_full_rows = Board::new();
        for row in [37, 38, 39] {
            for col in 0..BOARD_WIDTH as i32 {
                multiple_full_rows.set(row, col, Some(PieceKind::S));
            }
        }
        multiple_full_rows.set(36, 5, Some(PieceKind::Z));
        boards.push(multiple_full_rows);

        // Pseudo-random boards with scattered cells (not necessarily valid
        // Tetris stacks — the bitboard mirror must still be faithful).
        let mut rng = Rng::new(4242);
        for _ in 0..20 {
            let mut b = Board::new();
            for _ in 0..80 {
                let row = rng.next_below(BOARD_TOTAL_HEIGHT as u32) as i32;
                let col = rng.next_below(BOARD_WIDTH as u32) as i32;
                b.set(row, col, Some(PieceKind::O));
            }
            boards.push(b);
        }

        boards
    }

    #[test]
    fn column_height_matches_engine_board() {
        for board in engine_boards() {
            let bb = BitBoard::from_board(&board);
            for col in 0..BOARD_WIDTH {
                assert_eq!(bb.column_height(col), board.column_height(col), "column {col} mismatch\n{board:?}");
            }
        }
    }

    #[test]
    fn is_row_full_matches_engine_board() {
        for board in engine_boards() {
            let bb = BitBoard::from_board(&board);
            for row in 0..BOARD_TOTAL_HEIGHT {
                assert_eq!(bb.is_row_full(row), board.is_row_full(row), "row {row} mismatch\n{board:?}");
            }
        }
    }

    #[test]
    fn count_holes_matches_independent_engine_board_scan() {
        for board in engine_boards() {
            let bb = BitBoard::from_board(&board);
            // Independent reimplementation directly against engine::Board,
            // not a call to the same logic being tested.
            let mut expected = 0u32;
            for col in 0..BOARD_WIDTH as i32 {
                let mut found_filled = false;
                for row in 0..BOARD_TOTAL_HEIGHT as i32 {
                    if board.is_occupied(row, col) {
                        found_filled = true;
                    } else if found_filled {
                        expected += 1;
                    }
                }
            }
            assert_eq!(bb.count_holes(), expected, "hole count mismatch\n{board:?}");
        }
    }

    #[test]
    fn topmost_filled_row_matches_independent_scan() {
        for board in engine_boards() {
            let bb = BitBoard::from_board(&board);
            let expected = (0..BOARD_TOTAL_HEIGHT).find(|&r| !board.is_row_empty(r)).unwrap_or(BOARD_TOTAL_HEIGHT);
            assert_eq!(bb.topmost_filled_row(), expected, "topmost-filled-row mismatch\n{board:?}");
        }
    }

    #[test]
    fn clear_full_rows_matches_engine_board() {
        for mut board in engine_boards() {
            let mut bb = BitBoard::from_board(&board);
            let engine_cleared = board.clear_full_rows();
            let bitboard_cleared = bb.clear_full_rows();
            assert_eq!(bitboard_cleared, engine_cleared, "cleared-row count mismatch\n{board:?}");
            // Resulting boards must be equivalent cell-for-cell (colorless).
            for col in 0..BOARD_WIDTH {
                assert_eq!(bb.column_height(col), board.column_height(col), "post-clear column {col} mismatch");
            }
            for row in 0..BOARD_TOTAL_HEIGHT {
                for col in 0..BOARD_WIDTH as i32 {
                    assert_eq!(
                        bb.is_occupied(row as i32, col),
                        board.is_occupied(row as i32, col),
                        "post-clear cell ({row},{col}) mismatch"
                    );
                }
            }
        }
    }

    #[test]
    fn is_occupied_matches_engine_board_including_out_of_bounds() {
        for board in engine_boards() {
            let bb = BitBoard::from_board(&board);
            for row in -5..(BOARD_TOTAL_HEIGHT as i32 + 5) {
                for col in -3..(BOARD_WIDTH as i32 + 3) {
                    assert_eq!(bb.is_occupied(row, col), board.is_occupied(row, col), "cell ({row},{col}) mismatch");
                }
            }
        }
    }
}
