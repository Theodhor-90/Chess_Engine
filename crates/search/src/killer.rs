use chess_types::Move;

const MAX_PLY: usize = 128;
const NUM_KILLERS: usize = 2;

pub struct KillerTable {
    table: [[Option<Move>; NUM_KILLERS]; MAX_PLY],
}

impl Default for KillerTable {
    fn default() -> Self {
        Self::new()
    }
}

impl KillerTable {
    pub fn new() -> KillerTable {
        KillerTable {
            table: [[None; NUM_KILLERS]; MAX_PLY],
        }
    }

    pub fn store(&mut self, ply: u8, mv: Move) {
        let p = ply as usize;
        if self.table[p][0] == Some(mv) {
            return;
        }
        self.table[p][1] = self.table[p][0];
        self.table[p][0] = Some(mv);
    }

    pub fn is_killer(&self, ply: u8, mv: Move) -> bool {
        let p = ply as usize;
        self.table[p][0] == Some(mv) || self.table[p][1] == Some(mv)
    }

    pub fn clear(&mut self) {
        self.table = [[None; NUM_KILLERS]; MAX_PLY];
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chess_types::{MoveFlag, Square};

    fn quiet_move(from: u8, to: u8) -> Move {
        Move::new(
            Square::new(from).unwrap(),
            Square::new(to).unwrap(),
            MoveFlag::QUIET,
        )
    }

    #[test]
    fn store_and_retrieve() {
        let mut kt = KillerTable::new();
        let a = quiet_move(12, 28); // e2-e4
        let b = quiet_move(6, 21); // g1-f3

        kt.store(5, a);
        assert!(kt.is_killer(5, a));

        kt.store(5, b);
        assert!(kt.is_killer(5, a));
        assert!(kt.is_killer(5, b));
    }

    #[test]
    fn slot_rotation_evicts_oldest() {
        let mut kt = KillerTable::new();
        let a = quiet_move(12, 28);
        let b = quiet_move(6, 21);
        let c = quiet_move(1, 18);

        kt.store(3, a);
        kt.store(3, b);
        kt.store(3, c);

        assert!(kt.is_killer(3, c));
        assert!(kt.is_killer(3, b));
        assert!(!kt.is_killer(3, a));
    }

    #[test]
    fn duplicate_not_shifted() {
        let mut kt = KillerTable::new();
        let a = quiet_move(12, 28);
        let b = quiet_move(6, 21);

        kt.store(4, a);
        kt.store(4, b);
        kt.store(4, a);

        assert!(kt.is_killer(4, a));
        assert!(kt.is_killer(4, b));
    }

    #[test]
    fn killers_per_ply_independent() {
        let mut kt = KillerTable::new();
        let a = quiet_move(12, 28);
        let b = quiet_move(6, 21);

        kt.store(0, a);
        kt.store(1, b);

        assert!(kt.is_killer(0, a));
        assert!(!kt.is_killer(1, a));
        assert!(kt.is_killer(1, b));
        assert!(!kt.is_killer(0, b));
    }

    #[test]
    fn clear_resets_all() {
        let mut kt = KillerTable::new();
        let a = quiet_move(12, 28);
        let b = quiet_move(6, 21);
        let c = quiet_move(1, 18);

        kt.store(0, a);
        kt.store(5, b);
        kt.store(10, c);
        kt.clear();

        assert!(!kt.is_killer(0, a));
        assert!(!kt.is_killer(5, b));
        assert!(!kt.is_killer(10, c));
    }
}
