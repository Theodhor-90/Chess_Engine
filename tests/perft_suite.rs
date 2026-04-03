use chess_board::Position;
use chess_movegen::perft;

fn parse_perft_entries() -> Vec<(&'static str, u32, u64)> {
    let data = include_str!("data/perft_suite.txt");
    data.lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.starts_with('#')
        })
        .map(|line| {
            let parts: Vec<&str> = line.split(';').collect();
            assert_eq!(
                parts.len(),
                3,
                "malformed line (expected 3 semicolon-separated fields): {line}"
            );
            let fen = parts[0];
            let depth: u32 = parts[1].parse().unwrap_or_else(|_| {
                panic!("invalid depth '{}' in line: {line}", parts[1]);
            });
            let expected: u64 = parts[2].parse().unwrap_or_else(|_| {
                panic!("invalid node count '{}' in line: {line}", parts[2]);
            });
            (fen, depth, expected)
        })
        .collect()
}

#[test]
fn test_perft_suite() {
    let entries = parse_perft_entries();
    assert!(
        entries.len() >= 100,
        "perft suite has only {} entries, expected at least 100",
        entries.len()
    );

    let mut passed = 0usize;
    let mut failed = 0usize;
    let mut failures: Vec<String> = Vec::new();

    for (i, &(fen, depth, expected)) in entries.iter().enumerate() {
        let mut pos = Position::from_fen(fen).unwrap_or_else(|e| {
            panic!("[{i}] invalid FEN '{fen}': {e}");
        });
        let actual = perft(&mut pos, depth);
        if actual == expected {
            passed += 1;
        } else {
            failed += 1;
            failures.push(format!(
                "[{i}] FAIL: FEN='{fen}' depth={depth} expected={expected} actual={actual}"
            ));
        }
    }

    if failed > 0 {
        panic!(
            "{failed}/{} perft positions failed:\n{}",
            entries.len(),
            failures.join("\n")
        );
    }

    assert_eq!(passed, entries.len(), "not all entries were tested");
}
