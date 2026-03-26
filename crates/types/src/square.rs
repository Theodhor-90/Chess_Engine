/// A file (column) on the chess board.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[repr(u8)]
pub enum File {
    A = 0,
    B = 1,
    C = 2,
    D = 3,
    E = 4,
    F = 5,
    G = 6,
    H = 7,
}

impl File {
    /// Number of files.
    pub const COUNT: usize = 8;
}

impl TryFrom<u8> for File {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(File::A),
            1 => Ok(File::B),
            2 => Ok(File::C),
            3 => Ok(File::D),
            4 => Ok(File::E),
            5 => Ok(File::F),
            6 => Ok(File::G),
            7 => Ok(File::H),
            _ => Err("invalid file value"),
        }
    }
}

impl std::fmt::Display for File {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ch = (b'a' + *self as u8) as char;
        write!(f, "{ch}")
    }
}

/// A rank (row) on the chess board.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[repr(u8)]
pub enum Rank {
    R1 = 0,
    R2 = 1,
    R3 = 2,
    R4 = 3,
    R5 = 4,
    R6 = 5,
    R7 = 6,
    R8 = 7,
}

impl Rank {
    /// Number of ranks.
    pub const COUNT: usize = 8;
}

impl TryFrom<u8> for Rank {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Rank::R1),
            1 => Ok(Rank::R2),
            2 => Ok(Rank::R3),
            3 => Ok(Rank::R4),
            4 => Ok(Rank::R5),
            5 => Ok(Rank::R6),
            6 => Ok(Rank::R7),
            7 => Ok(Rank::R8),
            _ => Err("invalid rank value"),
        }
    }
}

impl std::fmt::Display for Rank {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ch = (b'1' + *self as u8) as char;
        write!(f, "{ch}")
    }
}

/// A square on the chess board, using little-endian rank-file mapping (a1=0, h8=63).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Square(u8);

impl Square {
    /// Number of squares.
    pub const COUNT: usize = 64;

    pub const A1: Square = Square(0);
    pub const B1: Square = Square(1);
    pub const C1: Square = Square(2);
    pub const D1: Square = Square(3);
    pub const E1: Square = Square(4);
    pub const F1: Square = Square(5);
    pub const G1: Square = Square(6);
    pub const H1: Square = Square(7);
    pub const A2: Square = Square(8);
    pub const B2: Square = Square(9);
    pub const C2: Square = Square(10);
    pub const D2: Square = Square(11);
    pub const E2: Square = Square(12);
    pub const F2: Square = Square(13);
    pub const G2: Square = Square(14);
    pub const H2: Square = Square(15);
    pub const A3: Square = Square(16);
    pub const B3: Square = Square(17);
    pub const C3: Square = Square(18);
    pub const D3: Square = Square(19);
    pub const E3: Square = Square(20);
    pub const F3: Square = Square(21);
    pub const G3: Square = Square(22);
    pub const H3: Square = Square(23);
    pub const A4: Square = Square(24);
    pub const B4: Square = Square(25);
    pub const C4: Square = Square(26);
    pub const D4: Square = Square(27);
    pub const E4: Square = Square(28);
    pub const F4: Square = Square(29);
    pub const G4: Square = Square(30);
    pub const H4: Square = Square(31);
    pub const A5: Square = Square(32);
    pub const B5: Square = Square(33);
    pub const C5: Square = Square(34);
    pub const D5: Square = Square(35);
    pub const E5: Square = Square(36);
    pub const F5: Square = Square(37);
    pub const G5: Square = Square(38);
    pub const H5: Square = Square(39);
    pub const A6: Square = Square(40);
    pub const B6: Square = Square(41);
    pub const C6: Square = Square(42);
    pub const D6: Square = Square(43);
    pub const E6: Square = Square(44);
    pub const F6: Square = Square(45);
    pub const G6: Square = Square(46);
    pub const H6: Square = Square(47);
    pub const A7: Square = Square(48);
    pub const B7: Square = Square(49);
    pub const C7: Square = Square(50);
    pub const D7: Square = Square(51);
    pub const E7: Square = Square(52);
    pub const F7: Square = Square(53);
    pub const G7: Square = Square(54);
    pub const H7: Square = Square(55);
    pub const A8: Square = Square(56);
    pub const B8: Square = Square(57);
    pub const C8: Square = Square(58);
    pub const D8: Square = Square(59);
    pub const E8: Square = Square(60);
    pub const F8: Square = Square(61);
    pub const G8: Square = Square(62);
    pub const H8: Square = Square(63);

    /// Creates a new square from an index (0..63). Returns `None` if out of range.
    pub fn new(index: u8) -> Option<Square> {
        if index > 63 {
            None
        } else {
            Some(Square(index))
        }
    }

    /// Returns the raw index.
    pub fn index(self) -> u8 {
        self.0
    }

    /// Returns the file of this square.
    pub fn file(self) -> File {
        File::try_from(self.0 % 8).expect("file always valid for valid square")
    }

    /// Returns the rank of this square.
    pub fn rank(self) -> Rank {
        Rank::try_from(self.0 / 8).expect("rank always valid for valid square")
    }

    /// Constructs a square from a file and rank.
    pub fn from_file_rank(file: File, rank: Rank) -> Square {
        Square(rank as u8 * 8 + file as u8)
    }
}

impl From<Square> for (File, Rank) {
    fn from(sq: Square) -> (File, Rank) {
        (sq.file(), sq.rank())
    }
}

impl From<(File, Rank)> for Square {
    fn from((file, rank): (File, Rank)) -> Square {
        Square::from_file_rank(file, rank)
    }
}

impl std::fmt::Display for Square {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.file(), self.rank())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_square_boundary_values() {
        assert_eq!(Square::A1.index(), 0);
        assert_eq!(Square::H1.index(), 7);
        assert_eq!(Square::A8.index(), 56);
        assert_eq!(Square::H8.index(), 63);
    }

    #[test]
    fn test_square_round_trip_all() {
        for i in 0..64u8 {
            let sq = Square::new(i).unwrap();
            let (file, rank) = <(File, Rank)>::from(sq);
            let sq2 = Square::from((file, rank));
            assert_eq!(sq, sq2);
        }
    }

    #[test]
    fn test_square_new_valid() {
        for i in 0..64u8 {
            assert!(Square::new(i).is_some());
        }
    }

    #[test]
    fn test_square_new_invalid() {
        assert!(Square::new(64).is_none());
        assert!(Square::new(255).is_none());
    }

    #[test]
    fn test_file_rank_extraction() {
        assert_eq!(Square::A1.file(), File::A);
        assert_eq!(Square::A1.rank(), Rank::R1);
        assert_eq!(Square::H8.file(), File::H);
        assert_eq!(Square::H8.rank(), Rank::R8);
        assert_eq!(Square::E4.file(), File::E);
        assert_eq!(Square::E4.rank(), Rank::R4);
    }

    #[test]
    fn test_file_try_from_valid() {
        for i in 0..8u8 {
            assert!(File::try_from(i).is_ok());
        }
    }

    #[test]
    fn test_file_try_from_invalid() {
        assert!(File::try_from(8).is_err());
    }

    #[test]
    fn test_rank_try_from_valid() {
        for i in 0..8u8 {
            assert!(Rank::try_from(i).is_ok());
        }
    }

    #[test]
    fn test_rank_try_from_invalid() {
        assert!(Rank::try_from(8).is_err());
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", Square::A1), "a1");
        assert_eq!(format!("{}", Square::H8), "h8");
        assert_eq!(format!("{}", Square::E4), "e4");
    }
}
