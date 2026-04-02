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
    game_history: Vec<u64>,
    search_handle: Option<JoinHandle<Option<Move>>>,
    stop_flag: Arc<AtomicBool>,
    pondering: bool,
    ponder_params: Option<chess_uci::GoParams>,
    book: Option<chess_engine::book::PolyglotBook>,
    book_mode: chess_engine::book::BookMode,
    syzygy_path: String,
    syzygy_probe_depth: u8,
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
        game_history: vec![Position::startpos().hash()],
        search_handle: None,
        stop_flag: Arc::new(AtomicBool::new(false)),
        pondering: false,
        ponder_params: None,
        book: None,
        book_mode: chess_engine::book::BookMode::BestMove,
        syzygy_path: String::new(),
        syzygy_probe_depth: 6,
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
                println!("{}", chess_uci::output::option_string("BookFile", ""));
                println!(
                    "{}",
                    chess_uci::output::option_combo(
                        "BookMode",
                        "bestmove",
                        &["bestmove", "weighted"]
                    )
                );
                println!("{}", chess_uci::output::option_string("SyzygyPath", ""));
                println!(
                    "{}",
                    chess_uci::output::option_spin("SyzygyProbeDepth", 6, 0, 7)
                );
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
                state.game_history = vec![Position::startpos().hash()];
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
                let mut game_history: Vec<u64> = vec![pos.hash()];
                for move_str in &moves {
                    if let Some(mv) = parse_uci_move(&mut pos, move_str) {
                        let _ = pos.make_move(mv);
                        game_history.push(pos.hash());
                    }
                }
                state.position = pos;
                state.game_history = game_history;
            }
            chess_uci::UciCommand::SetOption { name, value } => {
                match name.to_lowercase().as_str() {
                    "bookfile" => {
                        if let Some(path) = value {
                            if path.is_empty() {
                                state.book = None;
                            } else {
                                match chess_engine::book::PolyglotBook::from_file(&path) {
                                    Ok(b) => state.book = Some(b),
                                    Err(e) => {
                                        println!(
                                            "{}",
                                            chess_uci::output::info_string(&format!(
                                                "failed to load book: {e}"
                                            ))
                                        );
                                        io::stdout().flush().ok();
                                    }
                                }
                            }
                        } else {
                            state.book = None;
                        }
                    }
                    "bookmode" => {
                        if let Some(val) = value {
                            match val.to_lowercase().as_str() {
                                "bestmove" => {
                                    state.book_mode = chess_engine::book::BookMode::BestMove
                                }
                                "weighted" => {
                                    state.book_mode = chess_engine::book::BookMode::Weighted
                                }
                                _ => {}
                            }
                        }
                    }
                    "syzygypath" => {
                        if let Some(ref path) = value {
                            if path.is_empty() {
                                state.syzygy_path = String::new();
                            } else {
                                state.syzygy_path = path.clone();
                            }
                        } else {
                            state.syzygy_path = String::new();
                        }
                    }
                    "syzygyprobedepth" => {
                        if let Some(ref val) = value {
                            if let Ok(n) = val.parse::<u8>() {
                                state.syzygy_probe_depth = n.min(7);
                            }
                        }
                    }
                    _ => {}
                }
            }
            chess_uci::UciCommand::Go(params) => {
                stop_search(&mut state);

                if let Some(ref book) = state.book {
                    let key = chess_engine::book::polyglot_hash(&state.position);
                    let entries = book.probe(key);
                    if !entries.is_empty() {
                        if let Some(mv) = chess_engine::book::select_book_move(
                            &mut state.position,
                            entries,
                            state.book_mode,
                        ) {
                            println!(
                                "{}",
                                chess_uci::output::info_string(&format!("book move {mv}"))
                            );
                            println!("{}", chess_uci::output::bestmove(mv, None));
                            io::stdout().flush().ok();
                            continue;
                        }
                    }
                }

                state.stop_flag = Arc::new(AtomicBool::new(false));

                let no_limit = Duration::from_secs(86400);
                let limits = if params.ponder {
                    state.pondering = true;
                    state.ponder_params = Some(params);
                    chess_search::SearchLimits {
                        max_time: no_limit,
                        max_depth: None,
                        max_nodes: None,
                        stop_flag: Some(Arc::clone(&state.stop_flag)),
                    }
                } else if let Some(d) = params.depth {
                    chess_search::SearchLimits {
                        max_time: no_limit,
                        max_depth: Some(d),
                        max_nodes: None,
                        stop_flag: Some(Arc::clone(&state.stop_flag)),
                    }
                } else if let Some(n) = params.nodes {
                    chess_search::SearchLimits {
                        max_time: no_limit,
                        max_depth: None,
                        max_nodes: Some(n),
                        stop_flag: Some(Arc::clone(&state.stop_flag)),
                    }
                } else if let Some(mt) = params.movetime {
                    chess_search::SearchLimits {
                        max_time: Duration::from_millis(mt),
                        max_depth: None,
                        max_nodes: None,
                        stop_flag: Some(Arc::clone(&state.stop_flag)),
                    }
                } else if params.infinite {
                    chess_search::SearchLimits {
                        max_time: no_limit,
                        max_depth: None,
                        max_nodes: None,
                        stop_flag: Some(Arc::clone(&state.stop_flag)),
                    }
                } else if params.wtime.is_some() || params.btime.is_some() {
                    let side = state.position.side_to_move();
                    let budget = chess_uci::time::allocate_time(&params, side);
                    chess_search::SearchLimits {
                        max_time: budget,
                        max_depth: None,
                        max_nodes: None,
                        stop_flag: Some(Arc::clone(&state.stop_flag)),
                    }
                } else {
                    chess_search::SearchLimits {
                        max_time: no_limit,
                        max_depth: None,
                        max_nodes: None,
                        stop_flag: Some(Arc::clone(&state.stop_flag)),
                    }
                };

                let mut search_pos = state.position.clone();
                let game_history = state.game_history.clone();
                let mut tb_prober: Option<chess_engine::syzygy::LazySyzygyTablebase> =
                    if !state.syzygy_path.is_empty() {
                        Some(chess_engine::syzygy::LazySyzygyTablebase::new(
                            state.syzygy_path.clone(),
                            state.syzygy_probe_depth,
                        ))
                    } else {
                        None
                    };
                state.search_handle = Some(std::thread::spawn(move || {
                    let result = chess_search::search(
                        &mut search_pos,
                        limits,
                        &game_history,
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
                        tb_prober
                            .as_mut()
                            .map(|t| t as &mut dyn chess_search::TbProber),
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
            chess_uci::UciCommand::PonderHit => {
                if state.pondering {
                    if let Some(ref params) = state.ponder_params {
                        let side = state.position.side_to_move();
                        let budget = chess_uci::time::allocate_time(params, side);
                        let stop = Arc::clone(&state.stop_flag);
                        std::thread::spawn(move || {
                            std::thread::sleep(budget);
                            stop.store(true, Ordering::Relaxed);
                        });
                    }
                    state.pondering = false;
                }
            }
            chess_uci::UciCommand::Stop => {
                state.pondering = false;
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
