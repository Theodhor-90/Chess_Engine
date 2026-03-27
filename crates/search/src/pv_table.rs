use chess_types::Move;

const MAX_PLY: usize = 128;

pub struct PvTable {
    table: [[Option<Move>; MAX_PLY]; MAX_PLY],
    length: [usize; MAX_PLY],
}

impl Default for PvTable {
    fn default() -> Self {
        Self::new()
    }
}

impl PvTable {
    pub fn new() -> PvTable {
        PvTable {
            table: [[None; MAX_PLY]; MAX_PLY],
            length: [0; MAX_PLY],
        }
    }

    pub fn update(&mut self, ply: u8, mv: Move) {
        let p = ply as usize;
        let child = p + 1;
        self.table[p][0] = Some(mv);
        let child_len = self.length[child];
        for i in 0..child_len {
            self.table[p][i + 1] = self.table[child][i];
        }
        self.length[p] = 1 + child_len;
    }

    pub fn get_pv_move(&self, ply: u8) -> Option<Move> {
        self.table[ply as usize][0]
    }

    pub fn extract_pv(&self) -> Vec<Move> {
        self.table[0][..self.length[0]]
            .iter()
            .filter_map(|&mv| mv)
            .collect()
    }

    pub fn clear(&mut self) {
        for row in &mut self.table {
            for cell in row.iter_mut() {
                *cell = None;
            }
        }
        self.length = [0; MAX_PLY];
    }

    pub fn clear_ply(&mut self, ply: u8) {
        self.length[ply as usize] = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chess_types::{MoveFlag, Square};

    fn make_move(from: u8, to: u8) -> Move {
        Move::new(
            Square::new(from).unwrap(),
            Square::new(to).unwrap(),
            MoveFlag::QUIET,
        )
    }

    #[test]
    fn update_and_extract_single_ply() {
        let mut pv = PvTable::new();
        let move_a = make_move(12, 28);
        pv.update(0, move_a);
        assert_eq!(pv.extract_pv(), vec![move_a]);
        assert_eq!(pv.get_pv_move(0), Some(move_a));
    }

    #[test]
    fn update_chains_child_pv() {
        let mut pv = PvTable::new();
        let move_a = make_move(12, 28);
        let move_b = make_move(52, 36);
        pv.update(1, move_b);
        pv.update(0, move_a);
        assert_eq!(pv.extract_pv(), vec![move_a, move_b]);
    }

    #[test]
    fn clear_resets_table() {
        let mut pv = PvTable::new();
        let move_a = make_move(12, 28);
        pv.update(0, move_a);
        pv.clear();
        assert_eq!(pv.extract_pv(), Vec::<Move>::new());
        assert_eq!(pv.get_pv_move(0), None);
    }

    #[test]
    fn clear_ply_resets_single_ply() {
        let mut pv = PvTable::new();
        let move_a = make_move(12, 28);
        pv.update(0, move_a);
        pv.clear_ply(0);
        assert_eq!(pv.extract_pv(), Vec::<Move>::new());
    }
}
