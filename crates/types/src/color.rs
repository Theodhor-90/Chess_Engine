/// The two sides in a chess game.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[repr(u8)]
pub enum Color {
    White = 0,
    Black = 1,
}

impl Color {
    /// Number of color variants.
    pub const COUNT: usize = 2;

    /// Returns the opposite color.
    pub fn opposite(self) -> Color {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }
}

impl TryFrom<u8> for Color {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Color::White),
            1 => Ok(Color::Black),
            _ => Err("invalid color value"),
        }
    }
}

impl std::fmt::Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Color::White => write!(f, "White"),
            Color::Black => write!(f, "Black"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opposite() {
        assert_eq!(Color::White.opposite(), Color::Black);
        assert_eq!(Color::Black.opposite(), Color::White);
    }

    #[test]
    fn test_opposite_round_trip() {
        assert_eq!(Color::White.opposite().opposite(), Color::White);
        assert_eq!(Color::Black.opposite().opposite(), Color::Black);
    }

    #[test]
    fn test_discriminant_values() {
        assert_eq!(Color::White as u8, 0);
        assert_eq!(Color::Black as u8, 1);
    }

    #[test]
    fn test_try_from_valid() {
        assert_eq!(Color::try_from(0), Ok(Color::White));
        assert_eq!(Color::try_from(1), Ok(Color::Black));
    }

    #[test]
    fn test_try_from_invalid() {
        assert!(Color::try_from(2).is_err());
        assert!(Color::try_from(255).is_err());
    }
}
