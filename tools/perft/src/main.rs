use chess_board::Position;
use chess_movegen::{generate_legal_moves, perft};
use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        usage();
    }

    let mut divide = false;
    let mut rest = &args[..];
    if rest[0] == "--divide" {
        divide = true;
        rest = &rest[1..];
    }

    if rest.len() < 2 {
        usage();
    }

    let depth: u32 = match rest.last().unwrap().parse() {
        Ok(d) => d,
        Err(_) => usage(),
    };

    let fen = rest[..rest.len() - 1].join(" ");
    let mut pos = match Position::from_fen(&fen) {
        Ok(p) => p,
        Err(_) => {
            eprintln!("Error: invalid FEN string");
            usage()
        }
    };

    if divide {
        let moves = generate_legal_moves(&mut pos);
        let mut total = 0u64;
        for mv in moves {
            let undo = pos.make_move(mv);
            let count = if depth > 1 {
                perft(&mut pos, depth - 1)
            } else {
                1
            };
            pos.unmake_move(mv, undo);
            println!("{mv}: {count}");
            total += count;
        }
        println!("Total: {total}");
    } else {
        let count = perft(&mut pos, depth);
        println!("Perft({depth}) = {count}");
    }
}

fn usage() -> ! {
    eprintln!("Usage: perft [--divide] <fen> <depth>");
    process::exit(1);
}
