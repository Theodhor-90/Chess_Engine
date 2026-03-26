/// Bitboard representation — a set of squares encoded as a 64-bit integer.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
pub struct Bitboard(u64);

impl Bitboard {
    pub const EMPTY: Bitboard = Bitboard(0);
    pub const FULL: Bitboard = Bitboard(u64::MAX);

    pub fn new(val: u64) -> Self {
        Bitboard(val)
    }

    pub fn inner(self) -> u64 {
        self.0
    }

    pub fn pop_count(self) -> u32 {
        self.0.count_ones()
    }

    pub fn lsb(self) -> Option<u32> {
        if self.is_empty() {
            None
        } else {
            Some(self.0.trailing_zeros())
        }
    }

    pub fn msb(self) -> Option<u32> {
        if self.is_empty() {
            None
        } else {
            Some(63 - self.0.leading_zeros())
        }
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }
}

use std::fmt;
use std::ops::{
    BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not, Shl, ShlAssign, Shr,
    ShrAssign,
};

impl BitAnd for Bitboard {
    type Output = Bitboard;
    fn bitand(self, rhs: Bitboard) -> Bitboard {
        Bitboard(self.0 & rhs.0)
    }
}

impl BitOr for Bitboard {
    type Output = Bitboard;
    fn bitor(self, rhs: Bitboard) -> Bitboard {
        Bitboard(self.0 | rhs.0)
    }
}

impl BitXor for Bitboard {
    type Output = Bitboard;
    fn bitxor(self, rhs: Bitboard) -> Bitboard {
        Bitboard(self.0 ^ rhs.0)
    }
}

impl Not for Bitboard {
    type Output = Bitboard;
    fn not(self) -> Bitboard {
        Bitboard(!self.0)
    }
}

impl Shl<u32> for Bitboard {
    type Output = Bitboard;
    fn shl(self, rhs: u32) -> Bitboard {
        Bitboard(self.0 << rhs)
    }
}

impl Shr<u32> for Bitboard {
    type Output = Bitboard;
    fn shr(self, rhs: u32) -> Bitboard {
        Bitboard(self.0 >> rhs)
    }
}

impl BitAndAssign for Bitboard {
    fn bitand_assign(&mut self, rhs: Bitboard) {
        self.0 &= rhs.0;
    }
}

impl BitOrAssign for Bitboard {
    fn bitor_assign(&mut self, rhs: Bitboard) {
        self.0 |= rhs.0;
    }
}

impl BitXorAssign for Bitboard {
    fn bitxor_assign(&mut self, rhs: Bitboard) {
        self.0 ^= rhs.0;
    }
}

impl ShlAssign<u32> for Bitboard {
    fn shl_assign(&mut self, rhs: u32) {
        self.0 <<= rhs;
    }
}

impl ShrAssign<u32> for Bitboard {
    fn shr_assign(&mut self, rhs: u32) {
        self.0 >>= rhs;
    }
}

pub struct BitboardIter {
    bb: Bitboard,
}

impl IntoIterator for Bitboard {
    type Item = u32;
    type IntoIter = BitboardIter;
    fn into_iter(self) -> BitboardIter {
        BitboardIter { bb: self }
    }
}

impl Iterator for BitboardIter {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.bb.0 == 0 {
            None
        } else {
            let idx = self.bb.0.trailing_zeros();
            self.bb.0 &= self.bb.0 - 1;
            Some(idx)
        }
    }
}

impl fmt::Display for Bitboard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for rank in (0..8).rev() {
            for file in 0..8 {
                let sq = rank * 8 + file;
                if self.0 & (1u64 << sq) != 0 {
                    write!(f, "1")?;
                } else {
                    write!(f, ".")?;
                }
                if file < 7 {
                    write!(f, " ")?;
                }
            }
            if rank > 0 {
                writeln!(f)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitand() {
        assert_eq!(Bitboard(0xFF) & Bitboard(0x0F), Bitboard(0x0F));
    }

    #[test]
    fn test_bitor() {
        assert_eq!(Bitboard(0xF0) | Bitboard(0x0F), Bitboard(0xFF));
    }

    #[test]
    fn test_bitxor() {
        assert_eq!(Bitboard(0xFF) ^ Bitboard(0x0F), Bitboard(0xF0));
    }

    #[test]
    fn test_not() {
        assert_eq!(!Bitboard(0), Bitboard(u64::MAX));
    }

    #[test]
    fn test_shl() {
        assert_eq!(Bitboard(1) << 8, Bitboard(256));
    }

    #[test]
    fn test_shr() {
        assert_eq!(Bitboard(256) >> 8, Bitboard(1));
    }

    #[test]
    fn test_assign_ops() {
        let mut bb = Bitboard(0xFF);
        bb &= Bitboard(0x0F);
        assert_eq!(bb, Bitboard(0x0F));

        bb |= Bitboard(0xF0);
        assert_eq!(bb, Bitboard(0xFF));

        bb ^= Bitboard(0x0F);
        assert_eq!(bb, Bitboard(0xF0));

        let mut bb2 = Bitboard(1);
        bb2 <<= 8;
        assert_eq!(bb2, Bitboard(256));

        bb2 >>= 8;
        assert_eq!(bb2, Bitboard(1));
    }

    #[test]
    fn test_pop_count_empty() {
        assert_eq!(Bitboard::EMPTY.pop_count(), 0);
    }

    #[test]
    fn test_pop_count_single() {
        assert_eq!(Bitboard(1).pop_count(), 1);
    }

    #[test]
    fn test_pop_count_multiple() {
        assert_eq!(Bitboard(0b1010_1010).pop_count(), 4);
    }

    #[test]
    fn test_pop_count_full() {
        assert_eq!(Bitboard::FULL.pop_count(), 64);
    }

    #[test]
    fn test_lsb_empty() {
        assert_eq!(Bitboard::EMPTY.lsb(), None);
    }

    #[test]
    fn test_lsb_bit0() {
        assert_eq!(Bitboard(1).lsb(), Some(0));
    }

    #[test]
    fn test_lsb_bit63() {
        assert_eq!(Bitboard(1u64 << 63).lsb(), Some(63));
    }

    #[test]
    fn test_msb_empty() {
        assert_eq!(Bitboard::EMPTY.msb(), None);
    }

    #[test]
    fn test_msb_bit63() {
        assert_eq!(Bitboard(1u64 << 63).msb(), Some(63));
    }

    #[test]
    fn test_msb_bit0() {
        assert_eq!(Bitboard(1).msb(), Some(0));
    }

    #[test]
    fn test_iter_empty() {
        let indices: Vec<u32> = Bitboard::EMPTY.into_iter().collect();
        assert!(indices.is_empty());
    }

    #[test]
    fn test_iter_single() {
        let indices: Vec<u32> = Bitboard(1 << 5).into_iter().collect();
        assert_eq!(indices, vec![5]);
    }

    #[test]
    fn test_iter_multiple() {
        let indices: Vec<u32> = Bitboard(0b1001_0010).into_iter().collect();
        assert_eq!(indices, vec![1, 4, 7]);
    }

    #[test]
    fn test_iter_full() {
        let indices: Vec<u32> = Bitboard::FULL.into_iter().collect();
        let expected: Vec<u32> = (0..=63).collect();
        assert_eq!(indices, expected);
    }
}
