use chess_types::{Move, PieceKind, Square};

pub struct CounterMoveTable {
    table: [[Option<Move>; Square::COUNT]; PieceKind::COUNT],
}

impl Default for CounterMoveTable {
    fn default() -> Self {
        Self::new()
    }
}

impl CounterMoveTable {
    pub fn new() -> CounterMoveTable {
        CounterMoveTable {
            table: [[None; Square::COUNT]; PieceKind::COUNT],
        }
    }

    pub fn store(&mut self, piece: PieceKind, to: Square, counter_move: Move) {
        self.table[piece as usize][to.index() as usize] = Some(counter_move);
    }

    pub fn lookup(&self, piece: PieceKind, to: Square) -> Option<Move> {
        self.table[piece as usize][to.index() as usize]
    }

    pub fn clear(&mut self) {
        self.table = [[None; Square::COUNT]; PieceKind::COUNT];
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chess_types::MoveFlag;

    #[test]
    fn store_and_lookup_round_trip() {
        let mut table = CounterMoveTable::new();
        let counter = Move::new(Square::E2, Square::E4, MoveFlag::QUIET);
        table.store(PieceKind::Knight, Square::F3, counter);
        assert_eq!(table.lookup(PieceKind::Knight, Square::F3), Some(counter));
    }

    #[test]
    fn lookup_returns_none_initially() {
        let table = CounterMoveTable::new();
        assert_eq!(table.lookup(PieceKind::Pawn, Square::E4), None);
    }

    #[test]
    fn clear_resets_all_entries() {
        let mut table = CounterMoveTable::new();
        let mv1 = Move::new(Square::A2, Square::A4, MoveFlag::QUIET);
        let mv2 = Move::new(Square::B1, Square::C3, MoveFlag::QUIET);
        table.store(PieceKind::Pawn, Square::E4, mv1);
        table.store(PieceKind::Knight, Square::F3, mv2);
        table.clear();
        assert_eq!(table.lookup(PieceKind::Pawn, Square::E4), None);
        assert_eq!(table.lookup(PieceKind::Knight, Square::F3), None);
    }

    #[test]
    fn overwrite_replaces_previous() {
        let mut table = CounterMoveTable::new();
        let mv_a = Move::new(Square::A2, Square::A4, MoveFlag::QUIET);
        let mv_b = Move::new(Square::B1, Square::C3, MoveFlag::QUIET);
        table.store(PieceKind::Rook, Square::D4, mv_a);
        table.store(PieceKind::Rook, Square::D4, mv_b);
        assert_eq!(table.lookup(PieceKind::Rook, Square::D4), Some(mv_b));
    }
}
