use chess_types::{Color, Square};

pub struct HistoryTable {
    table: [[[i32; Square::COUNT]; Square::COUNT]; Color::COUNT],
}

impl Default for HistoryTable {
    fn default() -> Self {
        Self::new()
    }
}

impl HistoryTable {
    pub fn new() -> HistoryTable {
        HistoryTable {
            table: [[[0; Square::COUNT]; Square::COUNT]; Color::COUNT],
        }
    }

    pub fn update(&mut self, side: Color, from: Square, to: Square, bonus: i32) {
        let entry = &mut self.table[side as usize][from.index() as usize][to.index() as usize];
        *entry = *entry - *entry * bonus.abs() / 16384 + bonus;
    }

    pub fn score(&self, side: Color, from: Square, to: Square) -> i32 {
        self.table[side as usize][from.index() as usize][to.index() as usize]
    }

    pub fn clear(&mut self) {
        self.table = [[[0; Square::COUNT]; Square::COUNT]; Color::COUNT];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gravity_formula_bounds_scores() {
        let mut ht = HistoryTable::new();
        let side = Color::White;
        let from = Square::E2;
        let to = Square::E4;

        for _ in 0..1000 {
            ht.update(side, from, to, 100);
        }
        let score = ht.score(side, from, to);
        assert!(
            score <= 16384,
            "score after repeated positive bonus should be <= 16384, got {}",
            score
        );
        assert!(score > 0);

        let mut ht2 = HistoryTable::new();
        for _ in 0..1000 {
            ht2.update(side, from, to, -100);
        }
        let score2 = ht2.score(side, from, to);
        assert!(
            score2 >= -16384,
            "score after repeated negative bonus should be >= -16384, got {}",
            score2
        );
        assert!(score2 < 0);
    }

    #[test]
    fn clear_resets_all() {
        let mut ht = HistoryTable::new();
        ht.update(Color::White, Square::E2, Square::E4, 50);
        ht.update(Color::Black, Square::D7, Square::D5, 30);
        ht.clear();
        assert_eq!(ht.score(Color::White, Square::E2, Square::E4), 0);
        assert_eq!(ht.score(Color::Black, Square::D7, Square::D5), 0);
    }

    #[test]
    fn initial_score_is_zero() {
        let ht = HistoryTable::new();
        assert_eq!(ht.score(Color::White, Square::A1, Square::H8), 0);
        assert_eq!(ht.score(Color::Black, Square::E2, Square::E4), 0);
    }

    #[test]
    fn positive_bonus_increases_score() {
        let mut ht = HistoryTable::new();
        ht.update(Color::White, Square::E2, Square::E4, 9);
        assert!(
            ht.score(Color::White, Square::E2, Square::E4) > 0,
            "score after positive bonus should be positive"
        );
    }

    #[test]
    fn negative_bonus_decreases_score() {
        let mut ht = HistoryTable::new();
        ht.update(Color::White, Square::E2, Square::E4, -9);
        assert!(
            ht.score(Color::White, Square::E2, Square::E4) < 0,
            "score after negative bonus should be negative"
        );
    }
}
