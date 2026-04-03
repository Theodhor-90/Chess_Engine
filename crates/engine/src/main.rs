use std::io::{self, BufRead, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use chess_board::Position;
use chess_types::{File, Move, PieceKind, Rank, Square};

const ENGINE_NAME: &str = "ChessEngine";
const ENGINE_AUTHOR: &str = "Theodhor";

struct EngineState {
    position: Position,
    search_handle: Option<JoinHandle<Option<Move>>>,
    stop_flag: Arc<AtomicBool>,
}

fn parse_uci_move(pos: &mut Position, move_str: &str) -> Option<Move> {
    let bytes = move_str.as_bytes();
    if bytes.len() < 4 || bytes.len() > 5 {
        return None;
    }
    let from_file = bytes[0].wrapping_sub(b'a');
    let from_rank = bytes[1].wrapping_sub(b'1');
    let to_file = bytes[2].wrapping_sub(b'a');
    let to_rank = bytes[3].wrapping_sub(b'1');
    let from = Square::from_file_rank(
        File::try_from(from_file).ok()?,
        Rank::try_from(from_rank).ok()?,
    );
    let to = Square::from_file_rank(File::try_from(to_file).ok()?, Rank::try_from(to_rank).ok()?);
    let promo_char = bytes.get(4).copied();

    let legal_moves = chess_movegen::generate_legal_moves(pos);
    legal_moves.into_iter().find(|mv| {
        mv.from_sq() == from
            && mv.to_sq() == to
            && match promo_char {
                Some(b'q') => mv.promotion_piece() == Some(PieceKind::Queen),
                Some(b'r') => mv.promotion_piece() == Some(PieceKind::Rook),
                Some(b'b') => mv.promotion_piece() == Some(PieceKind::Bishop),
                Some(b'n') => mv.promotion_piece() == Some(PieceKind::Knight),
                _ => !mv.is_promotion(),
            }
    })
}

fn score_to_uci(score: i32) -> chess_uci::output::Score {
    if score > chess_search::MATE_SCORE - 100 {
        let ply = chess_search::MATE_SCORE - score;
        chess_uci::output::Score::Mate((ply + 1) / 2)
    } else if score < -(chess_search::MATE_SCORE - 100) {
        let ply = chess_search::MATE_SCORE + score;
        chess_uci::output::Score::Mate(-((ply + 1) / 2))
    } else {
        chess_uci::output::Score::Cp(score)
    }
}

fn stop_search(state: &mut EngineState) {
    state.stop_flag.store(true, Ordering::Relaxed);
    if let Some(handle) = state.search_handle.take() {
        let _ = handle.join();
    }
}

fn main() -> anyhow::Result<()> {
    let mut state = EngineState {
        position: Position::startpos(),
        search_handle: None,
        stop_flag: Arc::new(AtomicBool::new(false)),
    };

    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line?;
        let cmd = match chess_uci::parse(&line) {
            Ok(cmd) => cmd,
            Err(_) => continue,
        };

        match cmd {
            chess_uci::UciCommand::Uci => {
                println!("{}", chess_uci::output::id_name(ENGINE_NAME));
                println!("{}", chess_uci::output::id_author(ENGINE_AUTHOR));
                println!("{}", chess_uci::output::uciok());
                io::stdout().flush().ok();
            }
            chess_uci::UciCommand::IsReady => {
                if let Some(handle) = state.search_handle.take() {
                    let _ = handle.join();
                }
                println!("{}", chess_uci::output::readyok());
                io::stdout().flush().ok();
            }
            chess_uci::UciCommand::UciNewGame => {
                stop_search(&mut state);
                state.position = Position::startpos();
            }
            chess_uci::UciCommand::Position { fen, moves } => {
                let mut pos = if let Some(fen_str) = fen {
                    match Position::from_fen(&fen_str) {
                        Ok(p) => p,
                        Err(_) => continue,
                    }
                } else {
                    Position::startpos()
                };
                for move_str in &moves {
                    if let Some(mv) = parse_uci_move(&mut pos, move_str) {
                        let _ = pos.make_move(mv);
                    }
                }
                state.position = pos;
            }
            chess_uci::UciCommand::Go(params) => {
                stop_search(&mut state);
                state.stop_flag = Arc::new(AtomicBool::new(false));

                let time_budget = if let Some(mt) = params.movetime {
                    Duration::from_millis(mt)
                } else if params.infinite || params.depth.is_some() {
                    Duration::from_secs(86400)
                } else if params.wtime.is_some() || params.btime.is_some() {
                    Duration::from_secs(5)
                } else {
                    Duration::from_secs(86400)
                };

                let mut search_pos = state.position.clone();
                let stop = Arc::clone(&state.stop_flag);
                state.search_handle = Some(std::thread::spawn(move || {
                    let result = chess_search::search(
                        &mut search_pos,
                        time_budget,
                        Some(stop),
                        Some(&|depth, score, nodes, elapsed, pv| {
                            let time_ms = elapsed.as_millis() as u64;
                            let nps = if time_ms > 0 {
                                nodes * 1000 / time_ms
                            } else {
                                0
                            };
                            let uci_score = score_to_uci(score);
                            let info = chess_uci::output::InfoFields::new()
                                .depth(depth as u32)
                                .score(uci_score)
                                .nodes(nodes)
                                .nps(nps)
                                .time(time_ms)
                                .pv(pv.to_vec());
                            if let Some(line) = info.to_uci_string() {
                                println!("{line}");
                            }
                        }),
                    );
                    if let Some(mv) = result {
                        println!("{}", chess_uci::output::bestmove(mv, None));
                    } else {
                        println!("bestmove 0000");
                    }
                    io::stdout().flush().ok();
                    result
                }));
            }
            chess_uci::UciCommand::Stop => {
                stop_search(&mut state);
            }
            chess_uci::UciCommand::Quit => {
                stop_search(&mut state);
                return Ok(());
            }
        }
    }

    Ok(())
}
