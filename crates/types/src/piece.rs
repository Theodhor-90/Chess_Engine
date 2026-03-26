use crate::color::Color;

/// The six types of chess pieces.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[repr(u8)]
pub enum PieceKind {
    Pawn = 0,
    Knight = 1,
    Bishop = 2,
    Rook = 3,
    Queen = 4,
    King = 5,
}

impl PieceKind {
    /// Number of piece kind variants.
    pub const COUNT: usize = 6;
}

impl TryFrom<u8> for PieceKind {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(PieceKind::Pawn),
            1 => Ok(PieceKind::Knight),
            2 => Ok(PieceKind::Bishop),
            3 => Ok(PieceKind::Rook),
            4 => Ok(PieceKind::Queen),
            5 => Ok(PieceKind::King),
            _ => Err("invalid piece kind value"),
        }
    }
}

/// A chess piece with a color and kind.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Piece {
    pub color: Color,
    pub kind: PieceKind,
}

impl Piece {
    /// Creates a new piece.
    pub fn new(color: Color, kind: PieceKind) -> Piece {
        Piece { color, kind }
    }

    /// Returns a flat index in 0..12 for array indexing.
    pub fn index(self) -> usize {
        self.color as usize * 6 + self.kind as usize
    }

    /// Constructs a piece from a flat index (0..12). Returns `None` for invalid indices.
    pub fn from_index(index: usize) -> Option<Piece> {
        if index >= 12 {
            return None;
        }
        let color = Color::try_from((index / 6) as u8).ok()?;
        let kind = PieceKind::try_from((index % 6) as u8).ok()?;
        Some(Piece { color, kind })
    }
}

impl std::fmt::Display for Piece {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ch = match self.kind {
            PieceKind::Pawn => 'P',
            PieceKind::Knight => 'N',
            PieceKind::Bishop => 'B',
            PieceKind::Rook => 'R',
            PieceKind::Queen => 'Q',
            PieceKind::King => 'K',
        };
        let ch = match self.color {
            Color::White => ch,
            Color::Black => ch.to_ascii_lowercase(),
        };
        write!(f, "{ch}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_piece_kind_discriminants() {
        assert_eq!(PieceKind::Pawn as u8, 0);
        assert_eq!(PieceKind::Knight as u8, 1);
        assert_eq!(PieceKind::Bishop as u8, 2);
        assert_eq!(PieceKind::Rook as u8, 3);
        assert_eq!(PieceKind::Queen as u8, 4);
        assert_eq!(PieceKind::King as u8, 5);
    }

    #[test]
    fn test_piece_kind_try_from_valid() {
        for i in 0..6u8 {
            assert!(PieceKind::try_from(i).is_ok());
        }
    }

    #[test]
    fn test_piece_kind_try_from_invalid() {
        assert!(PieceKind::try_from(6).is_err());
    }

    #[test]
    fn test_piece_index_all() {
        let mut seen = [false; 12];
        let colors = [Color::White, Color::Black];
        let kinds = [
            PieceKind::Pawn,
            PieceKind::Knight,
            PieceKind::Bishop,
            PieceKind::Rook,
            PieceKind::Queen,
            PieceKind::King,
        ];
        for &color in &colors {
            for &kind in &kinds {
                let idx = Piece::new(color, kind).index();
                assert!(idx < 12);
                assert!(!seen[idx], "duplicate index {idx}");
                seen[idx] = true;
            }
        }
        assert!(seen.iter().all(|&s| s));
    }

    #[test]
    fn test_piece_from_index_round_trip() {
        let colors = [Color::White, Color::Black];
        let kinds = [
            PieceKind::Pawn,
            PieceKind::Knight,
            PieceKind::Bishop,
            PieceKind::Rook,
            PieceKind::Queen,
            PieceKind::King,
        ];
        for &color in &colors {
            for &kind in &kinds {
                let p = Piece::new(color, kind);
                assert_eq!(Piece::from_index(p.index()), Some(p));
            }
        }
    }

    #[test]
    fn test_piece_from_index_invalid() {
        assert_eq!(Piece::from_index(12), None);
        assert_eq!(Piece::from_index(255), None);
    }
}
