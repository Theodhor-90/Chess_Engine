pub mod output;
pub mod time;

#[derive(Debug, thiserror::Error)]
pub enum UciError {
    #[error("empty input")]
    EmptyInput,
    #[error("unknown command: '{0}'")]
    UnknownCommand(String),
    #[error("invalid position command: {0}")]
    InvalidPosition(String),
    #[error("invalid go parameter: {0}")]
    InvalidGoParam(String),
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct GoParams {
    pub wtime: Option<u64>,
    pub btime: Option<u64>,
    pub winc: Option<u64>,
    pub binc: Option<u64>,
    pub movestogo: Option<u32>,
    pub depth: Option<u8>,
    pub nodes: Option<u64>,
    pub movetime: Option<u64>,
    pub infinite: bool,
    pub ponder: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum UciCommand {
    Uci,
    IsReady,
    UciNewGame,
    Position {
        fen: Option<String>,
        moves: Vec<String>,
    },
    Go(GoParams),
    Stop,
    PonderHit,
    Quit,
}

fn parse_position(tokens: &[&str]) -> Result<UciCommand, UciError> {
    if tokens.is_empty() {
        return Err(UciError::InvalidPosition("missing subcommand".to_string()));
    }

    let (fen, rest) = if tokens[0] == "startpos" {
        (None, &tokens[1..])
    } else if tokens[0] == "fen" {
        if tokens.len() < 7 {
            return Err(UciError::InvalidPosition(
                "incomplete FEN string".to_string(),
            ));
        }
        let fen_string = tokens[1..7].join(" ");
        (Some(fen_string), &tokens[7..])
    } else {
        return Err(UciError::InvalidPosition(format!(
            "expected 'startpos' or 'fen', got '{}'",
            tokens[0]
        )));
    };

    let moves = if !rest.is_empty() && rest[0] == "moves" {
        rest[1..].iter().map(|s| (*s).to_string()).collect()
    } else {
        Vec::new()
    };

    Ok(UciCommand::Position { fen, moves })
}

fn parse_go(tokens: &[&str]) -> Result<UciCommand, UciError> {
    let mut params = GoParams::default();
    let mut i = 0;

    while i < tokens.len() {
        match tokens[i] {
            "infinite" => params.infinite = true,
            "ponder" => params.ponder = true,
            "wtime" => {
                i += 1;
                params.wtime = Some(parse_go_value::<u64>(tokens, i)?);
            }
            "btime" => {
                i += 1;
                params.btime = Some(parse_go_value::<u64>(tokens, i)?);
            }
            "winc" => {
                i += 1;
                params.winc = Some(parse_go_value::<u64>(tokens, i)?);
            }
            "binc" => {
                i += 1;
                params.binc = Some(parse_go_value::<u64>(tokens, i)?);
            }
            "movestogo" => {
                i += 1;
                params.movestogo = Some(parse_go_value::<u32>(tokens, i)?);
            }
            "depth" => {
                i += 1;
                params.depth = Some(parse_go_value::<u8>(tokens, i)?);
            }
            "nodes" => {
                i += 1;
                params.nodes = Some(parse_go_value::<u64>(tokens, i)?);
            }
            "movetime" => {
                i += 1;
                params.movetime = Some(parse_go_value::<u64>(tokens, i)?);
            }
            other => {
                return Err(UciError::InvalidGoParam(format!(
                    "unknown parameter '{other}'"
                )));
            }
        }
        i += 1;
    }

    Ok(UciCommand::Go(params))
}

fn parse_go_value<T: std::str::FromStr>(tokens: &[&str], i: usize) -> Result<T, UciError> {
    let token = tokens
        .get(i)
        .ok_or_else(|| UciError::InvalidGoParam("missing value".to_string()))?;
    token
        .parse::<T>()
        .map_err(|_| UciError::InvalidGoParam(format!("invalid value '{token}'")))
}

pub fn parse(input: &str) -> Result<UciCommand, UciError> {
    let tokens: Vec<&str> = input.split_whitespace().collect();

    if tokens.is_empty() {
        return Err(UciError::EmptyInput);
    }

    match tokens[0] {
        "uci" => Ok(UciCommand::Uci),
        "isready" => Ok(UciCommand::IsReady),
        "ucinewgame" => Ok(UciCommand::UciNewGame),
        "stop" => Ok(UciCommand::Stop),
        "ponderhit" => Ok(UciCommand::PonderHit),
        "quit" => Ok(UciCommand::Quit),
        "position" => parse_position(&tokens[1..]),
        "go" => parse_go(&tokens[1..]),
        other => Err(UciError::UnknownCommand(other.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_uci() {
        assert_eq!(parse("uci").unwrap(), UciCommand::Uci);
    }

    #[test]
    fn parse_isready() {
        assert_eq!(parse("isready").unwrap(), UciCommand::IsReady);
    }

    #[test]
    fn parse_ucinewgame() {
        assert_eq!(parse("ucinewgame").unwrap(), UciCommand::UciNewGame);
    }

    #[test]
    fn parse_stop() {
        assert_eq!(parse("stop").unwrap(), UciCommand::Stop);
    }

    #[test]
    fn parse_quit() {
        assert_eq!(parse("quit").unwrap(), UciCommand::Quit);
    }

    #[test]
    fn parse_position_startpos() {
        assert_eq!(
            parse("position startpos").unwrap(),
            UciCommand::Position {
                fen: None,
                moves: vec![],
            }
        );
    }

    #[test]
    fn parse_position_startpos_with_moves() {
        assert_eq!(
            parse("position startpos moves e2e4 e7e5").unwrap(),
            UciCommand::Position {
                fen: None,
                moves: vec!["e2e4".to_string(), "e7e5".to_string()],
            }
        );
    }

    #[test]
    fn parse_position_fen() {
        assert_eq!(
            parse("position fen rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap(),
            UciCommand::Position {
                fen: Some("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1".to_string()),
                moves: vec![],
            }
        );
    }

    #[test]
    fn parse_position_fen_with_moves() {
        assert_eq!(
            parse(
                "position fen rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1 moves e7e5 d2d4"
            )
            .unwrap(),
            UciCommand::Position {
                fen: Some(
                    "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1".to_string()
                ),
                moves: vec!["e7e5".to_string(), "d2d4".to_string()],
            }
        );
    }

    #[test]
    fn parse_go_all_params() {
        assert_eq!(
            parse("go wtime 300000 btime 300000 winc 2000 binc 2000 movestogo 40").unwrap(),
            UciCommand::Go(GoParams {
                wtime: Some(300000),
                btime: Some(300000),
                winc: Some(2000),
                binc: Some(2000),
                movestogo: Some(40),
                ..GoParams::default()
            })
        );
    }

    #[test]
    fn parse_go_some_params() {
        assert_eq!(
            parse("go wtime 60000 btime 60000").unwrap(),
            UciCommand::Go(GoParams {
                wtime: Some(60000),
                btime: Some(60000),
                ..GoParams::default()
            })
        );
    }

    #[test]
    fn parse_go_no_params() {
        assert_eq!(parse("go").unwrap(), UciCommand::Go(GoParams::default()));
    }

    #[test]
    fn parse_go_infinite() {
        assert_eq!(
            parse("go infinite").unwrap(),
            UciCommand::Go(GoParams {
                infinite: true,
                ..GoParams::default()
            })
        );
    }

    #[test]
    fn parse_go_depth() {
        assert_eq!(
            parse("go depth 5").unwrap(),
            UciCommand::Go(GoParams {
                depth: Some(5),
                ..GoParams::default()
            })
        );
    }

    #[test]
    fn parse_go_movetime() {
        assert_eq!(
            parse("go movetime 1000").unwrap(),
            UciCommand::Go(GoParams {
                movetime: Some(1000),
                ..GoParams::default()
            })
        );
    }

    #[test]
    fn parse_go_ponder() {
        assert_eq!(
            parse("go ponder").unwrap(),
            UciCommand::Go(GoParams {
                ponder: true,
                ..GoParams::default()
            })
        );
    }

    #[test]
    fn parse_empty_input() {
        assert!(matches!(parse(""), Err(UciError::EmptyInput)));
    }

    #[test]
    fn parse_whitespace_only() {
        assert!(matches!(parse("   "), Err(UciError::EmptyInput)));
    }

    #[test]
    fn parse_unknown_command() {
        let err = parse("invalid").unwrap_err();
        assert!(matches!(err, UciError::UnknownCommand(ref s) if s == "invalid"));
    }

    #[test]
    fn parse_position_missing_args() {
        assert!(matches!(
            parse("position"),
            Err(UciError::InvalidPosition(_))
        ));
    }

    #[test]
    fn parse_position_invalid_keyword() {
        assert!(matches!(
            parse("position badarg"),
            Err(UciError::InvalidPosition(_))
        ));
    }

    #[test]
    fn parse_go_invalid_value() {
        assert!(matches!(
            parse("go depth abc"),
            Err(UciError::InvalidGoParam(_))
        ));
    }

    #[test]
    fn parse_ponderhit() {
        assert_eq!(parse("ponderhit").unwrap(), UciCommand::PonderHit);
    }
}
