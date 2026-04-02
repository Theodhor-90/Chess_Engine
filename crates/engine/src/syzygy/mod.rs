use chess_board::Position;
use chess_search::MATE_SCORE;
use chess_types::{Color, Piece, PieceKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wdl {
    Loss,
    BlessedLoss,
    Draw,
    CursedWin,
    Win,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Dtz(pub i32);

pub struct SyzygyTablebase {
    fathom: fathom_syzygy::Fathom,
    max_pieces: u8,
}

/// TB win score: MATE_SCORE - 200 = 29800.
/// Places TB wins below actual checkmate scores (MATE_SCORE - ply, ply < MAX_PLY = 128)
/// but clearly in winning territory.
const TB_WIN_SCORE: i32 = MATE_SCORE - 200;

impl SyzygyTablebase {
    pub fn init(path: &str) -> Result<SyzygyTablebase, String> {
        let fathom = fathom_syzygy::Fathom::new(path)
            .map_err(|e| format!("Failed to initialize Syzygy tablebases: {e}"))?;
        let max_pieces = fathom.max_pieces() as u8;
        Ok(SyzygyTablebase { fathom, max_pieces })
    }

    pub fn max_pieces(&self) -> u8 {
        self.max_pieces
    }

    pub fn probe_wdl(&mut self, pos: &Position, probe_limit: u8) -> Option<Wdl> {
        if piece_count(pos) > probe_limit || piece_count(pos) > self.max_pieces {
            return None;
        }
        let fathom_pos = to_fathom_position(pos);
        let (_, prober) = self.fathom.get_probers();
        let wdl = prober.probe(&fathom_pos)?;
        Some(convert_wdl(wdl))
    }

    pub fn probe_dtz(&mut self, pos: &Position, probe_limit: u8) -> Option<Dtz> {
        if piece_count(pos) > probe_limit || piece_count(pos) > self.max_pieces {
            return None;
        }
        let fathom_pos = to_fathom_position(pos);
        let (mut root_prober, _) = self.fathom.get_probers();
        let result = root_prober.probe(&fathom_pos)?;
        Some(Dtz(result.dtz as i32))
    }
}

pub fn wdl_to_score(wdl: Wdl) -> i32 {
    match wdl {
        Wdl::Win => TB_WIN_SCORE,
        Wdl::CursedWin => 1,
        Wdl::Draw => 0,
        Wdl::BlessedLoss => -1,
        Wdl::Loss => -TB_WIN_SCORE,
    }
}

fn piece_count(pos: &Position) -> u8 {
    pos.occupied().pop_count() as u8
}

fn to_fathom_position(pos: &Position) -> fathom_syzygy::Position {
    let bb = |c: Color, k: PieceKind| -> u64 { pos.piece_bitboard(Piece::new(c, k)).inner() };

    fathom_syzygy::Position {
        white: pos.occupied_by(Color::White).inner(),
        black: pos.occupied_by(Color::Black).inner(),
        kings: bb(Color::White, PieceKind::King) | bb(Color::Black, PieceKind::King),
        queens: bb(Color::White, PieceKind::Queen) | bb(Color::Black, PieceKind::Queen),
        rooks: bb(Color::White, PieceKind::Rook) | bb(Color::Black, PieceKind::Rook),
        bishops: bb(Color::White, PieceKind::Bishop) | bb(Color::Black, PieceKind::Bishop),
        knights: bb(Color::White, PieceKind::Knight) | bb(Color::Black, PieceKind::Knight),
        pawns: bb(Color::White, PieceKind::Pawn) | bb(Color::Black, PieceKind::Pawn),
        rule50: pos.halfmove_clock() as u32,
        castling: 0,
        ep: pos.en_passant().map(|sq| sq.index() as u32).unwrap_or(0),
        turn: if pos.side_to_move() == Color::White {
            1
        } else {
            0
        },
    }
}

fn convert_wdl(wdl: fathom_syzygy::Wdl) -> Wdl {
    match wdl {
        fathom_syzygy::Wdl::Loss => Wdl::Loss,
        fathom_syzygy::Wdl::BlessedLoss => Wdl::BlessedLoss,
        fathom_syzygy::Wdl::Draw => Wdl::Draw,
        fathom_syzygy::Wdl::CursedWin => Wdl::CursedWin,
        fathom_syzygy::Wdl::Win => Wdl::Win,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chess_board::Position;

    #[test]
    fn wdl_to_score_win() {
        assert_eq!(wdl_to_score(Wdl::Win), 29800);
    }

    #[test]
    fn wdl_to_score_loss() {
        assert_eq!(wdl_to_score(Wdl::Loss), -29800);
    }

    #[test]
    fn wdl_to_score_draw() {
        assert_eq!(wdl_to_score(Wdl::Draw), 0);
    }

    #[test]
    fn wdl_to_score_cursed_win() {
        assert_eq!(wdl_to_score(Wdl::CursedWin), 1);
    }

    #[test]
    fn wdl_to_score_blessed_loss() {
        assert_eq!(wdl_to_score(Wdl::BlessedLoss), -1);
    }

    #[test]
    fn piece_count_startpos() {
        let pos =
            Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
        assert_eq!(piece_count(&pos), 32);
    }

    #[test]
    fn piece_count_endgame() {
        let pos = Position::from_fen("8/8/8/8/8/4K3/8/2k1R3 w - - 0 1").unwrap();
        assert_eq!(piece_count(&pos), 3);
    }

    #[test]
    fn probe_wdl_rejects_too_many_pieces() {
        let pos =
            Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
        let mut tb = SyzygyTablebase {
            fathom: fathom_syzygy::Fathom::new("/nonexistent")
                .unwrap_or_else(|_| panic!("fathom init")),
            max_pieces: 6,
        };
        // 32 pieces > probe_limit of 6 → should return None without probing
        let result = tb.probe_wdl(&pos, 6);
        assert!(result.is_none());
    }

    #[test]
    #[ignore]
    fn probe_wdl_krk_win() {
        let path = std::env::var("SYZYGY_PATH").expect("Set SYZYGY_PATH to run this test");
        let mut tb = SyzygyTablebase::init(&path).expect("Failed to init tablebases");
        let pos = Position::from_fen("8/8/8/8/8/4K3/8/2k1R3 w - - 0 1").unwrap();
        let wdl = tb.probe_wdl(&pos, 6).expect("WDL probe failed");
        assert_eq!(wdl, Wdl::Win);
    }

    #[test]
    #[ignore]
    fn probe_dtz_krk() {
        let path = std::env::var("SYZYGY_PATH").expect("Set SYZYGY_PATH to run this test");
        let mut tb = SyzygyTablebase::init(&path).expect("Failed to init tablebases");
        let pos = Position::from_fen("8/8/8/8/8/4K3/8/2k1R3 w - - 0 1").unwrap();
        let dtz = tb.probe_dtz(&pos, 6).expect("DTZ probe failed");
        assert!(dtz.0 >= 0, "DTZ should be non-negative for winning side");
    }

    #[test]
    #[ignore]
    fn probe_wdl_kk_draw() {
        let path = std::env::var("SYZYGY_PATH").expect("Set SYZYGY_PATH to run this test");
        let mut tb = SyzygyTablebase::init(&path).expect("Failed to init tablebases");
        let pos = Position::from_fen("8/8/8/4k3/8/4K3/8/8 w - - 0 1").unwrap();
        let wdl = tb.probe_wdl(&pos, 6).expect("WDL probe failed");
        assert_eq!(wdl, Wdl::Draw);
    }
}
