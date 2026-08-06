#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum PieceKind {
    I,
    J,
    L,
    O,
    S,
    Z,
    T,
}

impl PieceKind {
    pub const ALL: [PieceKind; 7] = [
        PieceKind::I,
        PieceKind::J,
        PieceKind::L,
        PieceKind::O,
        PieceKind::S,
        PieceKind::Z,
        PieceKind::T,
    ];

    /// 1..=7, 0 reserved for "empty" in flat buffers.
    pub fn as_u8(self) -> u8 {
        match self {
            PieceKind::I => 1,
            PieceKind::J => 2,
            PieceKind::L => 3,
            PieceKind::O => 4,
            PieceKind::S => 5,
            PieceKind::Z => 6,
            PieceKind::T => 7,
        }
    }

    /// Side length of this piece's rotation bounding box (3 for JLSTZ, 4 for I; O uses 2).
    pub fn box_size(self) -> i32 {
        match self {
            PieceKind::I => 4,
            PieceKind::O => 2,
            _ => 3,
        }
    }

    /// Column (relative to the board) of the bounding box's top-left at spawn,
    /// chosen so the piece is horizontally centered on a 10-wide board.
    pub fn spawn_col(self) -> i32 {
        match self {
            PieceKind::O => 4,
            _ => 3,
        }
    }
}

/// SRS rotation states, in the order they cycle under a clockwise rotation.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Rotation {
    R0,
    R,
    R2,
    L,
}

impl Rotation {
    pub fn cw(self) -> Rotation {
        match self {
            Rotation::R0 => Rotation::R,
            Rotation::R => Rotation::R2,
            Rotation::R2 => Rotation::L,
            Rotation::L => Rotation::R0,
        }
    }

    pub fn ccw(self) -> Rotation {
        match self {
            Rotation::R0 => Rotation::L,
            Rotation::L => Rotation::R2,
            Rotation::R2 => Rotation::R,
            Rotation::R => Rotation::R0,
        }
    }
}

/// An active, falling piece. `row`/`col` locate the top-left of its rotation
/// bounding box in board coordinates (may be negative or extend into hidden rows).
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct ActivePiece {
    pub kind: PieceKind,
    pub rotation: Rotation,
    pub row: i32,
    pub col: i32,
}

impl ActivePiece {
    pub fn spawn(kind: PieceKind, spawn_row: i32) -> Self {
        ActivePiece {
            kind,
            rotation: Rotation::R0,
            row: spawn_row,
            col: kind.spawn_col(),
        }
    }

    /// Cell offsets (row, col) occupied by this piece, relative to (self.row, self.col).
    pub fn cells(&self) -> [(i32, i32); 4] {
        crate::rotation::shape(self.kind, self.rotation)
    }

    /// Absolute board (row, col) coordinates occupied by this piece.
    pub fn occupied_cells(&self) -> [(i32, i32); 4] {
        let mut out = [(0, 0); 4];
        for (i, (dr, dc)) in self.cells().iter().enumerate() {
            out[i] = (self.row + dr, self.col + dc);
        }
        out
    }

    pub fn with_rotation(&self, rotation: Rotation, row: i32, col: i32) -> ActivePiece {
        ActivePiece {
            kind: self.kind,
            rotation,
            row,
            col,
        }
    }
}
