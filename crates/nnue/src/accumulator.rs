use chess_board::Position;
use chess_types::{Color, Move, Piece, PieceKind, Square};

use crate::arch::L1_SIZE;
use crate::feature::feature_index;
use crate::inference::forward;
use crate::network::Network;
use crate::simd;

/// Stores the incrementally updatable state for hidden layer 1.
///
/// Each perspective (white and black) has its own accumulator vector of `L1_SIZE`
/// elements. The accumulator is designed to support incremental updates: features
/// can be added or removed without full recomputation.
#[derive(Clone)]
pub struct Accumulator {
    /// White perspective accumulator values (L1_SIZE elements).
    pub white: [i16; L1_SIZE],
    /// Black perspective accumulator values (L1_SIZE elements).
    pub black: [i16; L1_SIZE],
}

impl Accumulator {
    /// Creates a zeroed accumulator.
    pub fn new() -> Self {
        Self {
            white: [0; L1_SIZE],
            black: [0; L1_SIZE],
        }
    }

    /// Sets both perspectives to the bias values (used when refreshing from scratch).
    pub fn init_from_bias(&mut self, bias: &[i16; L1_SIZE]) {
        self.white = *bias;
        self.black = *bias;
    }

    /// Adds the weight column at `index` to the given perspective's accumulator.
    ///
    /// For each `i in 0..L1_SIZE`, adds `weights[index * L1_SIZE + i]` to the
    /// perspective's accumulator value at position `i`.
    pub fn add_feature(&mut self, perspective: Color, index: usize, weights: &[i16]) {
        let vals = match perspective {
            Color::White => &mut self.white,
            Color::Black => &mut self.black,
        };
        let offset = index * L1_SIZE;
        simd::vec_add_i16(vals, &weights[offset..offset + L1_SIZE]);
    }

    /// Subtracts the weight column at `index` from the given perspective's accumulator.
    ///
    /// For each `i in 0..L1_SIZE`, subtracts `weights[index * L1_SIZE + i]` from the
    /// perspective's accumulator value at position `i`.
    pub fn remove_feature(&mut self, perspective: Color, index: usize, weights: &[i16]) {
        let vals = match perspective {
            Color::White => &mut self.white,
            Color::Black => &mut self.black,
        };
        let offset = index * L1_SIZE;
        simd::vec_sub_i16(vals, &weights[offset..offset + L1_SIZE]);
    }

    /// Initializes the accumulator from a full position.
    ///
    /// Sets both perspectives to the L1 bias, then iterates all non-king pieces
    /// on the board and adds their corresponding HalfKP feature weight columns.
    pub fn refresh(&mut self, pos: &Position, network: &Network) {
        self.init_from_bias(&network.input_bias);

        let non_king_kinds = [
            PieceKind::Pawn,
            PieceKind::Knight,
            PieceKind::Bishop,
            PieceKind::Rook,
            PieceKind::Queen,
        ];

        for perspective in [Color::White, Color::Black] {
            let king_piece = Piece::new(perspective, PieceKind::King);
            let king_sq = match pos.piece_bitboard(king_piece).lsb() {
                Some(idx) => Square::new(idx as u8).expect("valid king square"),
                None => continue,
            };

            for &piece_color in &[Color::White, Color::Black] {
                for &piece_kind in &non_king_kinds {
                    let piece = Piece::new(piece_color, piece_kind);
                    for sq_idx in pos.piece_bitboard(piece) {
                        let piece_sq = Square::new(sq_idx as u8).expect("valid square");
                        let idx = feature_index(king_sq, piece_sq, piece_kind, piece_color);
                        self.add_feature(perspective, idx, &network.input_weights);
                    }
                }
            }
        }
    }
}

impl Default for Accumulator {
    fn default() -> Self {
        Self::new()
    }
}

pub struct AccumulatorState {
    pub acc: Accumulator,
    dirty: [bool; 2],
    king_moved: bool,
}

impl AccumulatorState {
    pub fn new() -> Self {
        Self {
            acc: Accumulator::new(),
            dirty: [false; 2],
            king_moved: false,
        }
    }

    pub fn from_accumulator(acc: Accumulator) -> Self {
        Self {
            acc,
            dirty: [false; 2],
            king_moved: false,
        }
    }

    pub fn mark_dirty(&mut self, perspective: Color) {
        self.dirty[perspective as usize] = true;
    }

    pub fn mark_king_moved(&mut self) {
        self.king_moved = true;
    }

    pub fn is_dirty(&self, perspective: Color) -> bool {
        self.dirty[perspective as usize]
    }

    pub fn needs_refresh(&self) -> bool {
        self.king_moved
    }
}

impl Default for AccumulatorState {
    fn default() -> Self {
        Self::new()
    }
}

fn king_sq_for(pos: &Position, perspective: Color) -> Square {
    let king_piece = Piece::new(perspective, PieceKind::King);
    let idx = pos
        .piece_bitboard(king_piece)
        .lsb()
        .expect("king must exist");
    Square::new(idx as u8).expect("valid king square")
}

fn compute_and_apply_deltas(
    state: &mut AccumulatorState,
    pos: &Position,
    mv: Move,
    captured: Option<Piece>,
    network: &Network,
) {
    // pos is the position AFTER make_move, so side_to_move is the opponent of the moving side
    let moving_side = pos.side_to_move().opposite();
    let from = mv.from_sq();
    let to = mv.to_sq();

    // Determine the moving piece kind. After make_move, the piece is at `to` (unless promotion,
    // where it changed kind). For promotion moves, the piece at `to` is the promoted piece,
    // but the original piece was a pawn.
    let is_promotion = mv.is_promotion();
    let is_castling = mv.is_castling();
    let is_en_passant = mv.is_en_passant();

    let piece_at_to = pos.piece_on(to).expect("piece must be at to_sq after move");
    let moving_kind = if is_promotion {
        PieceKind::Pawn
    } else {
        piece_at_to.kind
    };

    // If king moved, mark for full refresh
    if moving_kind == PieceKind::King || is_castling {
        state.mark_king_moved();
        return;
    }

    // For non-king moves, apply incremental deltas for each perspective
    for perspective in [Color::White, Color::Black] {
        let king_sq = king_sq_for(pos, perspective);

        // Remove feature at from_sq for the original piece
        let remove_idx = feature_index(king_sq, from, moving_kind, moving_side);
        state
            .acc
            .remove_feature(perspective, remove_idx, &network.input_weights);

        // Add feature at to_sq for the piece (or promoted piece)
        let add_kind = if is_promotion {
            piece_at_to.kind
        } else {
            moving_kind
        };
        let add_idx = feature_index(king_sq, to, add_kind, moving_side);
        state
            .acc
            .add_feature(perspective, add_idx, &network.input_weights);

        // Handle captured piece
        if let Some(cap) = captured {
            if is_en_passant {
                // En passant: captured pawn is not on to_sq, it's one rank behind
                let cap_sq_idx = if moving_side == Color::White {
                    to.index() - 8
                } else {
                    to.index() + 8
                };
                let cap_sq = Square::new(cap_sq_idx).expect("valid en passant capture square");
                let cap_idx = feature_index(king_sq, cap_sq, cap.kind, cap.color);
                state
                    .acc
                    .remove_feature(perspective, cap_idx, &network.input_weights);
            } else {
                let cap_idx = feature_index(king_sq, to, cap.kind, cap.color);
                state
                    .acc
                    .remove_feature(perspective, cap_idx, &network.input_weights);
            }
        }
    }
}

pub struct AccumulatorStack {
    stack: Vec<AccumulatorState>,
}

impl AccumulatorStack {
    pub fn new() -> Self {
        Self {
            stack: vec![AccumulatorState {
                acc: Accumulator::new(),
                dirty: [true; 2],
                king_moved: false,
            }],
        }
    }

    pub fn current(&self) -> &AccumulatorState {
        self.stack.last().expect("stack must not be empty")
    }

    pub fn current_mut(&mut self) -> &mut AccumulatorState {
        self.stack.last_mut().expect("stack must not be empty")
    }

    pub fn push_move(
        &mut self,
        pos: &Position,
        mv: Move,
        captured: Option<Piece>,
        network: &Network,
    ) {
        let mut new_state = AccumulatorState {
            acc: self.current().acc.clone(),
            dirty: self.current().dirty,
            king_moved: false,
        };
        compute_and_apply_deltas(&mut new_state, pos, mv, captured, network);
        self.stack.push(new_state);
    }

    pub fn pop(&mut self) {
        assert!(
            self.stack.len() > 1,
            "cannot pop the root accumulator state"
        );
        self.stack.pop();
    }

    pub fn refresh_if_needed(&mut self, pos: &Position, network: &Network) {
        let state = self.current_mut();
        if state.needs_refresh() || state.dirty[0] || state.dirty[1] {
            state.acc.refresh(pos, network);
            state.dirty = [false; 2];
            state.king_moved = false;
        }
    }

    pub fn evaluate(&mut self, pos: &Position, network: &Network, side_to_move: Color) -> i32 {
        self.refresh_if_needed(pos, network);
        forward(&self.current().acc, network, side_to_move)
    }
}

impl Default for AccumulatorStack {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arch::L1_SIZE;
    use chess_types::MoveFlag;

    fn test_network() -> Network {
        let mut net = Network::new_zeroed();
        for (i, w) in net.input_weights.iter_mut().enumerate() {
            *w = (i % 256) as i16 - 128;
        }
        let mut bias = [0i16; L1_SIZE];
        for (i, b) in bias.iter_mut().enumerate() {
            *b = (i as i16) % 50 - 25;
        }
        *net.input_bias = bias;
        net
    }

    #[test]
    fn accumulator_init_from_bias() {
        let mut acc = Accumulator::new();
        let mut bias = [0i16; L1_SIZE];
        for (i, b) in bias.iter_mut().enumerate() {
            *b = (i as i16) * 3 - 100;
        }
        acc.init_from_bias(&bias);
        assert_eq!(acc.white, bias);
        assert_eq!(acc.black, bias);
    }

    #[test]
    fn accumulator_add_remove_roundtrip() {
        let mut net = Network::new_zeroed();
        let feature_idx = 42;
        for i in 0..L1_SIZE {
            net.input_weights[feature_idx * L1_SIZE + i] = (i as i16) * 7 - 50;
        }

        let mut acc = Accumulator::new();
        acc.init_from_bias(&net.input_bias);
        let snapshot_white = acc.white;
        let snapshot_black = acc.black;

        acc.add_feature(Color::White, feature_idx, &net.input_weights);
        assert_ne!(acc.white, snapshot_white);
        acc.remove_feature(Color::White, feature_idx, &net.input_weights);
        assert_eq!(acc.white, snapshot_white);

        acc.add_feature(Color::Black, feature_idx, &net.input_weights);
        assert_ne!(acc.black, snapshot_black);
        acc.remove_feature(Color::Black, feature_idx, &net.input_weights);
        assert_eq!(acc.black, snapshot_black);
    }

    #[test]
    fn accumulator_refresh_startpos() {
        let mut net = Network::new_zeroed();
        // Set deterministic non-zero input weights.
        for (i, w) in net.input_weights.iter_mut().enumerate() {
            *w = (i % 256) as i16 - 128;
        }
        let mut bias = [0i16; L1_SIZE];
        for (i, b) in bias.iter_mut().enumerate() {
            *b = (i as i16) % 50 - 25;
        }
        *net.input_bias = bias;

        let pos = Position::startpos();
        let mut acc = Accumulator::new();
        acc.refresh(&pos, &net);

        // Manually compute expected accumulator for both perspectives.
        let non_king_kinds = [
            PieceKind::Pawn,
            PieceKind::Knight,
            PieceKind::Bishop,
            PieceKind::Rook,
            PieceKind::Queen,
        ];

        for perspective in [Color::White, Color::Black] {
            let mut expected = bias;
            let king_piece = Piece::new(perspective, PieceKind::King);
            let king_sq_idx = pos.piece_bitboard(king_piece).lsb().unwrap();
            let king_sq = Square::new(king_sq_idx as u8).unwrap();

            for &piece_color in &[Color::White, Color::Black] {
                for &piece_kind in &non_king_kinds {
                    let piece = Piece::new(piece_color, piece_kind);
                    for sq_idx in pos.piece_bitboard(piece) {
                        let piece_sq = Square::new(sq_idx as u8).unwrap();
                        let fi = feature_index(king_sq, piece_sq, piece_kind, piece_color);
                        let offset = fi * L1_SIZE;
                        for i in 0..L1_SIZE {
                            expected[i] += net.input_weights[offset + i];
                        }
                    }
                }
            }

            let actual = match perspective {
                Color::White => &acc.white,
                Color::Black => &acc.black,
            };
            assert_eq!(
                actual, &expected,
                "Accumulator mismatch for {:?} perspective",
                perspective
            );
        }

        // Verify that the accumulator was actually populated (not just bias).
        assert_ne!(acc.white, bias, "White accumulator should differ from bias");
        assert_ne!(acc.black, bias, "Black accumulator should differ from bias");
    }

    fn assert_acc_matches_refresh(stack: &AccumulatorStack, pos: &Position, network: &Network) {
        let mut expected = Accumulator::new();
        expected.refresh(pos, network);
        let actual = &stack.current().acc;
        assert_eq!(
            actual.white, expected.white,
            "White perspective mismatch after incremental update"
        );
        assert_eq!(
            actual.black, expected.black,
            "Black perspective mismatch after incremental update"
        );
    }

    fn assert_acc_matches_refresh_after_king_refresh(
        stack: &mut AccumulatorStack,
        pos: &Position,
        network: &Network,
    ) {
        stack.refresh_if_needed(pos, network);
        let mut expected = Accumulator::new();
        expected.refresh(pos, network);
        let actual = &stack.current().acc;
        assert_eq!(
            actual.white, expected.white,
            "White perspective mismatch after refresh"
        );
        assert_eq!(
            actual.black, expected.black,
            "Black perspective mismatch after refresh"
        );
    }

    #[test]
    fn incremental_matches_full_refresh_quiet_move() {
        let net = test_network();
        // Start from startpos, play e2e4 (double pawn push, non-king non-capture)
        let mut pos = Position::startpos();
        let mut stack = AccumulatorStack::new();
        stack.refresh_if_needed(&pos, &net);

        let moves = chess_movegen::generate_legal_moves(&mut pos);
        // Find e2e4
        let mv = moves
            .iter()
            .find(|m| m.from_sq() == Square::E2 && m.to_sq() == Square::E4)
            .copied()
            .expect("e2e4 must be legal");

        let undo = pos.make_move(mv);
        stack.push_move(&pos, mv, undo.captured, &net);
        assert_acc_matches_refresh(&stack, &pos, &net);

        pos.unmake_move(mv, undo);
    }

    #[test]
    fn incremental_matches_full_refresh_capture() {
        let net = test_network();
        // Position with a capture available
        let mut pos =
            Position::from_fen("rnbqkbnr/ppp1pppp/8/3p4/4P3/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 2")
                .expect("valid fen");
        let mut stack = AccumulatorStack::new();
        stack.refresh_if_needed(&pos, &net);

        let moves = chess_movegen::generate_legal_moves(&mut pos);
        // Find exd5
        let mv = moves
            .iter()
            .find(|m| m.from_sq() == Square::E4 && m.to_sq() == Square::D5 && m.is_capture())
            .copied()
            .expect("exd5 must be legal");

        let undo = pos.make_move(mv);
        stack.push_move(&pos, mv, undo.captured, &net);
        assert_acc_matches_refresh(&stack, &pos, &net);

        pos.unmake_move(mv, undo);
    }

    #[test]
    fn incremental_matches_full_refresh_en_passant() {
        let net = test_network();
        // Position with en passant available
        let mut pos =
            Position::from_fen("rnbqkbnr/pppp1ppp/8/4pP2/8/8/PPPPP1PP/RNBQKBNR w KQkq e6 0 3")
                .expect("valid fen");
        let mut stack = AccumulatorStack::new();
        stack.refresh_if_needed(&pos, &net);

        let moves = chess_movegen::generate_legal_moves(&mut pos);
        let mv = moves
            .iter()
            .find(|m| m.is_en_passant())
            .copied()
            .expect("en passant must be legal");

        let undo = pos.make_move(mv);
        stack.push_move(&pos, mv, undo.captured, &net);
        assert_acc_matches_refresh(&stack, &pos, &net);

        pos.unmake_move(mv, undo);
    }

    #[test]
    fn incremental_matches_full_refresh_castling() {
        let net = test_network();
        // Position where white can castle king-side
        let mut pos = Position::from_fen(
            "r1bqk2r/ppppbppp/2n2n2/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 4 4",
        )
        .expect("valid fen");
        let mut stack = AccumulatorStack::new();
        stack.refresh_if_needed(&pos, &net);

        let moves = chess_movegen::generate_legal_moves(&mut pos);
        let mv = moves
            .iter()
            .find(|m| m.is_castling() && m.flags() == MoveFlag::KING_CASTLE)
            .copied()
            .expect("king-side castling must be legal");

        let undo = pos.make_move(mv);
        stack.push_move(&pos, mv, undo.captured, &net);
        // Castling involves king move -> needs refresh
        assert_acc_matches_refresh_after_king_refresh(&mut stack, &pos, &net);

        stack.pop();
        pos.unmake_move(mv, undo);

        // Now test queen-side castling
        let mut pos2 =
            Position::from_fen("r3kbnr/pppqpppp/2n5/3p1b2/3P1B2/2N5/PPPQPPPP/R3KBNR w KQkq - 6 5")
                .expect("valid fen");
        let mut stack2 = AccumulatorStack::new();
        stack2.refresh_if_needed(&pos2, &net);

        let moves2 = chess_movegen::generate_legal_moves(&mut pos2);
        let mv2 = moves2
            .iter()
            .find(|m| m.is_castling() && m.flags() == MoveFlag::QUEEN_CASTLE)
            .copied()
            .expect("queen-side castling must be legal");

        let undo2 = pos2.make_move(mv2);
        stack2.push_move(&pos2, mv2, undo2.captured, &net);
        assert_acc_matches_refresh_after_king_refresh(&mut stack2, &pos2, &net);

        pos2.unmake_move(mv2, undo2);
    }

    #[test]
    fn incremental_matches_full_refresh_promotion() {
        let net = test_network();
        // Position where white can promote
        let mut pos = Position::from_fen("8/4P3/8/8/8/8/8/4K2k w - - 0 1").expect("valid fen");
        let mut stack = AccumulatorStack::new();
        stack.refresh_if_needed(&pos, &net);

        let moves = chess_movegen::generate_legal_moves(&mut pos);
        let mv = moves
            .iter()
            .find(|m| m.is_promotion() && !m.is_capture())
            .copied()
            .expect("promotion must be legal");

        let undo = pos.make_move(mv);
        stack.push_move(&pos, mv, undo.captured, &net);
        assert_acc_matches_refresh(&stack, &pos, &net);

        stack.pop();
        pos.unmake_move(mv, undo);

        // Test promotion with capture
        let mut pos2 = Position::from_fen("3r4/4P3/8/8/8/8/8/4K2k w - - 0 1").expect("valid fen");
        let mut stack2 = AccumulatorStack::new();
        stack2.refresh_if_needed(&pos2, &net);

        let moves2 = chess_movegen::generate_legal_moves(&mut pos2);
        let mv2 = moves2
            .iter()
            .find(|m| m.is_promotion() && m.is_capture())
            .copied()
            .expect("promotion capture must be legal");

        let undo2 = pos2.make_move(mv2);
        stack2.push_move(&pos2, mv2, undo2.captured, &net);
        assert_acc_matches_refresh(&stack2, &pos2, &net);

        pos2.unmake_move(mv2, undo2);
    }

    #[test]
    fn stack_push_pop_restores_state() {
        let net = test_network();
        let mut pos = Position::startpos();
        let mut stack = AccumulatorStack::new();
        stack.refresh_if_needed(&pos, &net);

        let before_white = stack.current().acc.white;
        let before_black = stack.current().acc.black;

        let moves = chess_movegen::generate_legal_moves(&mut pos);
        let mv = moves[0];
        let undo = pos.make_move(mv);
        stack.push_move(&pos, mv, undo.captured, &net);

        // State should have changed
        // (Not necessarily, if the move doesn't affect the accumulator... but with non-zero weights it will)

        stack.pop();
        pos.unmake_move(mv, undo);

        assert_eq!(
            stack.current().acc.white,
            before_white,
            "White perspective not restored after push/pop"
        );
        assert_eq!(
            stack.current().acc.black,
            before_black,
            "Black perspective not restored after push/pop"
        );
    }

    #[test]
    fn incremental_chain_matches_refresh() {
        let net = test_network();
        let mut pos = Position::startpos();
        let mut stack = AccumulatorStack::new();
        stack.refresh_if_needed(&pos, &net);

        // Play a sequence of moves from startpos
        let move_sequence = [
            ("e2", "e4"),
            ("e7", "e5"),
            ("g1", "f3"),
            ("b8", "c6"),
            ("f1", "b5"),
            ("a7", "a6"),
            ("b5", "a4"),
            ("g8", "f6"),
            ("d2", "d3"),
            ("f8", "e7"),
        ];

        let mut undos = Vec::new();
        let mut played_moves = Vec::new();
        for (from_name, to_name) in &move_sequence {
            let from = square_from_name(from_name);
            let to = square_from_name(to_name);
            let moves = chess_movegen::generate_legal_moves(&mut pos);
            let mv = moves
                .iter()
                .find(|m| m.from_sq() == from && m.to_sq() == to)
                .copied()
                .unwrap_or_else(|| {
                    panic!("Move {from_name}{to_name} not found in legal moves");
                });
            let undo = pos.make_move(mv);
            stack.push_move(&pos, mv, undo.captured, &net);
            stack.refresh_if_needed(&pos, &net);
            undos.push(undo);
            played_moves.push(mv);
        }

        // After 10+ moves, compare to full refresh
        let mut expected = Accumulator::new();
        expected.refresh(&pos, &net);
        assert_eq!(
            stack.current().acc.white,
            expected.white,
            "Chain: white perspective mismatch"
        );
        assert_eq!(
            stack.current().acc.black,
            expected.black,
            "Chain: black perspective mismatch"
        );

        // Unwind
        for (mv, undo) in played_moves.into_iter().rev().zip(undos.into_iter().rev()) {
            stack.pop();
            pos.unmake_move(mv, undo);
        }
    }

    #[test]
    fn dirty_flag_tracks_correctly() {
        let net = test_network();
        let mut pos = Position::startpos();
        let mut stack = AccumulatorStack::new();
        stack.refresh_if_needed(&pos, &net);

        // After refresh, not dirty
        assert!(!stack.current().is_dirty(Color::White));
        assert!(!stack.current().is_dirty(Color::Black));
        assert!(!stack.current().needs_refresh());

        // Non-king move: state pushed, not king_moved
        let moves = chess_movegen::generate_legal_moves(&mut pos);
        let mv = moves
            .iter()
            .find(|m| m.from_sq() == Square::E2 && m.to_sq() == Square::E4)
            .copied()
            .expect("e2e4 must be legal");
        let undo = pos.make_move(mv);
        stack.push_move(&pos, mv, undo.captured, &net);
        assert!(!stack.current().needs_refresh());

        stack.pop();
        pos.unmake_move(mv, undo);

        // King move: needs_refresh should be true
        let mut pos2 =
            Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQK2R w KQkq - 0 1")
                .expect("valid fen");
        let mut stack2 = AccumulatorStack::new();
        stack2.refresh_if_needed(&pos2, &net);

        let moves2 = chess_movegen::generate_legal_moves(&mut pos2);
        // Find Kf1 (king move)
        let king_mv = moves2
            .iter()
            .find(|m| {
                let piece = pos2.piece_on(m.from_sq());
                piece.map_or(false, |p| p.kind == PieceKind::King) && !m.is_castling()
            })
            .copied()
            .expect("king move must exist");

        let undo2 = pos2.make_move(king_mv);
        stack2.push_move(&pos2, king_mv, undo2.captured, &net);
        assert!(stack2.current().needs_refresh());

        stack2.refresh_if_needed(&pos2, &net);
        assert!(!stack2.current().needs_refresh());

        pos2.unmake_move(king_mv, undo2);
    }

    fn square_from_name(name: &str) -> Square {
        let bytes = name.as_bytes();
        let file = bytes[0] - b'a';
        let rank = bytes[1] - b'1';
        Square::new(rank * 8 + file).expect("valid square")
    }

    #[test]
    fn accumulator_add_remove_matches_scalar_reference() {
        let mut net = Network::new_zeroed();
        for (i, w) in net.input_weights.iter_mut().enumerate() {
            *w = (i % 256) as i16 - 128;
        }
        *net.input_bias = [0i16; L1_SIZE];

        let feature_idx = 100;
        let mut acc = Accumulator::new();
        acc.init_from_bias(&net.input_bias);

        // Compute expected result with scalar arithmetic
        let offset = feature_idx * L1_SIZE;
        let mut expected_white = acc.white;
        for i in 0..L1_SIZE {
            expected_white[i] += net.input_weights[offset + i];
        }

        // Use the actual add_feature (which now dispatches to SIMD)
        acc.add_feature(Color::White, feature_idx, &net.input_weights);
        assert_eq!(
            acc.white, expected_white,
            "SIMD add_feature must match scalar"
        );

        // Now remove and verify we return to original
        acc.remove_feature(Color::White, feature_idx, &net.input_weights);
        assert_eq!(
            acc.white, *net.input_bias,
            "SIMD remove_feature must round-trip"
        );
    }
}
