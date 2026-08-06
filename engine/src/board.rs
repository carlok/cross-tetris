use crate::piece::PieceKind;

pub const BOARD_WIDTH: usize = 10;
pub const BOARD_VISIBLE_HEIGHT: usize = 20;
pub const BOARD_HIDDEN_ROWS: usize = 20;
pub const BOARD_TOTAL_HEIGHT: usize = BOARD_VISIBLE_HEIGHT + BOARD_HIDDEN_ROWS;

/// A cell is empty, or filled by the kind of piece that locked into it.
pub type Cell = Option<PieceKind>;

/// Row 0 is the topmost hidden row; row `BOARD_TOTAL_HEIGHT - 1` is the bottom visible row.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Board {
    cells: Vec<Cell>,
}

impl Board {
    pub fn new() -> Self {
        Board {
            cells: vec![None; BOARD_WIDTH * BOARD_TOTAL_HEIGHT],
        }
    }

    #[inline]
    fn index(row: i32, col: i32) -> Option<usize> {
        if row < 0 || col < 0 || row as usize >= BOARD_TOTAL_HEIGHT || col as usize >= BOARD_WIDTH
        {
            None
        } else {
            Some(row as usize * BOARD_WIDTH + col as usize)
        }
    }

    /// Returns true if (row, col) is within the board bounds.
    pub fn in_bounds(row: i32, col: i32) -> bool {
        Self::index(row, col).is_some()
    }

    /// Returns the cell at (row, col). Out-of-bounds cells (other than above the
    /// top, which is treated as empty/open) are treated as occupied so pieces
    /// cannot move off the board.
    pub fn get(&self, row: i32, col: i32) -> Cell {
        match Self::index(row, col) {
            Some(i) => self.cells[i],
            None => {
                if row < 0 {
                    None
                } else {
                    Some(PieceKind::I) // any Some(_) marks "occupied"; kind is irrelevant for collision
                }
            }
        }
    }

    pub fn set(&mut self, row: i32, col: i32, value: Cell) {
        if let Some(i) = Self::index(row, col) {
            self.cells[i] = value;
        }
    }

    pub fn is_occupied(&self, row: i32, col: i32) -> bool {
        self.get(row, col).is_some()
    }

    pub fn is_row_full(&self, row: usize) -> bool {
        (0..BOARD_WIDTH).all(|col| self.cells[row * BOARD_WIDTH + col].is_some())
    }

    pub fn is_row_empty(&self, row: usize) -> bool {
        (0..BOARD_WIDTH).all(|col| self.cells[row * BOARD_WIDTH + col].is_none())
    }

    /// Clears all full rows, collapsing rows above downward. Returns the number cleared.
    pub fn clear_full_rows(&mut self) -> u32 {
        let mut write_row = BOARD_TOTAL_HEIGHT as i32 - 1;
        let mut cleared = 0u32;
        for read_row in (0..BOARD_TOTAL_HEIGHT as i32).rev() {
            if self.is_row_full(read_row as usize) {
                cleared += 1;
                continue;
            }
            if write_row != read_row {
                for col in 0..BOARD_WIDTH {
                    let v = self.cells[read_row as usize * BOARD_WIDTH + col];
                    self.cells[write_row as usize * BOARD_WIDTH + col] = v;
                }
            }
            write_row -= 1;
        }
        while write_row >= 0 {
            for col in 0..BOARD_WIDTH {
                self.cells[write_row as usize * BOARD_WIDTH + col] = None;
            }
            write_row -= 1;
        }
        cleared
    }

    /// Height of the visible board (rows above the highest filled cell within
    /// visible rows), one column at a time. Index 0..BOARD_WIDTH.
    pub fn column_height(&self, col: usize) -> u32 {
        for row in 0..BOARD_TOTAL_HEIGHT {
            if self.cells[row * BOARD_WIDTH + col].is_some() {
                return (BOARD_TOTAL_HEIGHT - row) as u32;
            }
        }
        0
    }

    /// Flat visible-area buffer (row-major, top-to-bottom of the visible 20 rows),
    /// one byte per cell: 0 = empty, else 1..=7 for PieceKind.
    pub fn visible_buffer(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(BOARD_WIDTH * BOARD_VISIBLE_HEIGHT);
        for row in BOARD_HIDDEN_ROWS..BOARD_TOTAL_HEIGHT {
            for col in 0..BOARD_WIDTH {
                buf.push(match self.cells[row * BOARD_WIDTH + col] {
                    None => 0,
                    Some(kind) => kind.as_u8(),
                });
            }
        }
        buf
    }
}

impl Default for Board {
    fn default() -> Self {
        Self::new()
    }
}
