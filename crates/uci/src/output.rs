use chess_types::Move;

/// UCI score representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Score {
    /// Centipawn score.
    Cp(i32),
    /// Mate in N moves (positive = engine mates, negative = engine is mated).
    Mate(i32),
}

/// Builder for UCI `info` output lines.
#[derive(Debug, Clone, Default)]
pub struct InfoFields {
    pub depth: Option<u32>,
    pub score: Option<Score>,
    pub nodes: Option<u64>,
    pub nps: Option<u64>,
    pub time: Option<u64>,
    pub pv: Vec<Move>,
}

impl InfoFields {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn depth(mut self, depth: u32) -> Self {
        self.depth = Some(depth);
        self
    }

    pub fn score(mut self, score: Score) -> Self {
        self.score = Some(score);
        self
    }

    pub fn nodes(mut self, nodes: u64) -> Self {
        self.nodes = Some(nodes);
        self
    }

    pub fn nps(mut self, nps: u64) -> Self {
        self.nps = Some(nps);
        self
    }

    pub fn time(mut self, time: u64) -> Self {
        self.time = Some(time);
        self
    }

    pub fn pv(mut self, pv: Vec<Move>) -> Self {
        self.pv = pv;
        self
    }

    /// Formats the info line. Returns `None` if no fields are set.
    pub fn to_uci_string(&self) -> Option<String> {
        let mut parts: Vec<String> = Vec::new();

        if let Some(d) = self.depth {
            parts.push(format!("depth {d}"));
        }
        if let Some(ref s) = self.score {
            match s {
                Score::Cp(cp) => parts.push(format!("score cp {cp}")),
                Score::Mate(m) => parts.push(format!("score mate {m}")),
            }
        }
        if let Some(n) = self.nodes {
            parts.push(format!("nodes {n}"));
        }
        if let Some(n) = self.nps {
            parts.push(format!("nps {n}"));
        }
        if let Some(t) = self.time {
            parts.push(format!("time {t}"));
        }
        if !self.pv.is_empty() {
            let pv_str: Vec<String> = self.pv.iter().map(|m| m.to_string()).collect();
            parts.push(format!("pv {}", pv_str.join(" ")));
        }

        if parts.is_empty() {
            return None;
        }

        Some(format!("info {}", parts.join(" ")))
    }
}

/// Returns `"id name <name>"`.
pub fn id_name(name: &str) -> String {
    format!("id name {name}")
}

/// Returns `"id author <author>"`.
pub fn id_author(author: &str) -> String {
    format!("id author {author}")
}

/// Returns `"uciok"`.
pub fn uciok() -> String {
    "uciok".to_string()
}

/// Returns `"readyok"`.
pub fn readyok() -> String {
    "readyok".to_string()
}

pub fn option_string(name: &str, default: &str) -> String {
    format!("option name {name} type string default {default}")
}

pub fn option_combo(name: &str, default: &str, vars: &[&str]) -> String {
    let var_str: String = vars.iter().map(|v| format!(" var {v}")).collect();
    format!("option name {name} type combo default {default}{var_str}")
}

pub fn info_string(msg: &str) -> String {
    format!("info string {msg}")
}

/// Returns `"bestmove <move>"` or `"bestmove <move> ponder <move>"`.
pub fn bestmove(mv: Move, ponder: Option<Move>) -> String {
    match ponder {
        Some(p) => format!("bestmove {mv} ponder {p}"),
        None => format!("bestmove {mv}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chess_types::{MoveFlag, Square};

    #[test]
    fn test_id_name() {
        assert_eq!(id_name("Foo"), "id name Foo");
    }

    #[test]
    fn test_id_author() {
        assert_eq!(id_author("Bar"), "id author Bar");
    }

    #[test]
    fn test_uciok() {
        assert_eq!(uciok(), "uciok");
    }

    #[test]
    fn test_readyok() {
        assert_eq!(readyok(), "readyok");
    }

    #[test]
    fn test_bestmove_no_ponder() {
        let mv = Move::new(Square::E2, Square::E4, MoveFlag::QUIET);
        assert_eq!(bestmove(mv, None), "bestmove e2e4");
    }

    #[test]
    fn test_bestmove_with_ponder() {
        let mv = Move::new(Square::E2, Square::E4, MoveFlag::QUIET);
        let ponder_mv = Move::new(Square::E7, Square::E5, MoveFlag::QUIET);
        assert_eq!(bestmove(mv, Some(ponder_mv)), "bestmove e2e4 ponder e7e5");
    }

    #[test]
    fn test_bestmove_promotion() {
        let mv = Move::new(Square::A7, Square::A8, MoveFlag::QUEEN_PROMOTION);
        assert_eq!(bestmove(mv, None), "bestmove a7a8q");
    }

    #[test]
    fn test_info_depth_only() {
        assert_eq!(
            InfoFields::new().depth(5).to_uci_string(),
            Some("info depth 5".to_string())
        );
    }

    #[test]
    fn test_info_depth_score_nodes() {
        let result = InfoFields::new()
            .depth(10)
            .score(Score::Cp(35))
            .nodes(12345)
            .to_uci_string();
        assert_eq!(
            result,
            Some("info depth 10 score cp 35 nodes 12345".to_string())
        );
    }

    #[test]
    fn test_info_score_cp_negative() {
        let result = InfoFields::new().score(Score::Cp(-150)).to_uci_string();
        assert!(result.unwrap().contains("score cp -150"));
    }

    #[test]
    fn test_info_score_mate_positive() {
        let result = InfoFields::new().score(Score::Mate(3)).to_uci_string();
        assert!(result.unwrap().contains("score mate 3"));
    }

    #[test]
    fn test_info_score_mate_negative() {
        let result = InfoFields::new().score(Score::Mate(-2)).to_uci_string();
        assert!(result.unwrap().contains("score mate -2"));
    }

    #[test]
    fn test_info_full_line() {
        let pv = vec![
            Move::new(Square::E2, Square::E4, MoveFlag::QUIET),
            Move::new(Square::E7, Square::E5, MoveFlag::QUIET),
            Move::new(Square::G1, Square::F3, MoveFlag::QUIET),
        ];
        let result = InfoFields::new()
            .depth(15)
            .score(Score::Cp(100))
            .nodes(50000)
            .nps(1000000)
            .time(50)
            .pv(pv)
            .to_uci_string();
        assert_eq!(
            result,
            Some(
                "info depth 15 score cp 100 nodes 50000 nps 1000000 time 50 pv e2e4 e7e5 g1f3"
                    .to_string()
            )
        );
    }

    #[test]
    fn test_info_pv_multiple_moves() {
        let pv = vec![
            Move::new(Square::E2, Square::E4, MoveFlag::QUIET),
            Move::new(Square::E7, Square::E5, MoveFlag::QUIET),
            Move::new(Square::D2, Square::D4, MoveFlag::QUIET),
        ];
        let result = InfoFields::new().pv(pv).to_uci_string();
        assert!(result.unwrap().contains("pv e2e4 e7e5 d2d4"));
    }

    #[test]
    fn test_info_no_fields() {
        assert_eq!(InfoFields::new().to_uci_string(), None);
    }

    #[test]
    fn test_info_nps_only() {
        assert_eq!(
            InfoFields::new().nps(500000).to_uci_string(),
            Some("info nps 500000".to_string())
        );
    }

    #[test]
    fn test_info_time_only() {
        assert_eq!(
            InfoFields::new().time(1500).to_uci_string(),
            Some("info time 1500".to_string())
        );
    }

    #[test]
    fn test_option_string() {
        assert_eq!(
            option_string("BookFile", ""),
            "option name BookFile type string default "
        );
    }

    #[test]
    fn test_option_combo() {
        assert_eq!(
            option_combo("BookMode", "bestmove", &["bestmove", "weighted"]),
            "option name BookMode type combo default bestmove var bestmove var weighted"
        );
    }

    #[test]
    fn test_info_string() {
        assert_eq!(info_string("book move e2e4"), "info string book move e2e4");
    }
}
