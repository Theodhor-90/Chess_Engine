use std::time::Duration;

use chess_types::Color;

use crate::GoParams;

/// Computes the time budget for the current move.
///
/// Formula: `time_left / moves_to_go + increment`
/// - `moves_to_go` defaults to 25 when `GoParams::movestogo` is `None`
/// - `time_left` is `wtime` for `Color::White`, `btime` for `Color::Black`
/// - `increment` is `winc` for `Color::White`, `binc` for `Color::Black`
pub fn allocate_time(params: &GoParams, side: Color) -> Duration {
    let time_left = match side {
        Color::White => params.wtime,
        Color::Black => params.btime,
    };

    let time_left_ms = match time_left {
        Some(t) => t,
        None => return Duration::from_secs(1),
    };

    let increment_ms = match side {
        Color::White => params.winc.unwrap_or(0),
        Color::Black => params.binc.unwrap_or(0),
    };

    let moves_to_go = params.movestogo.unwrap_or(25) as u64;

    let budget_ms = time_left_ms / moves_to_go + increment_ms;
    let budget_ms = budget_ms.max(1);

    Duration::from_millis(budget_ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sudden_death_no_increment() {
        let params = GoParams {
            wtime: Some(300_000),
            btime: Some(300_000),
            ..GoParams::default()
        };
        assert_eq!(
            allocate_time(&params, Color::White),
            Duration::from_millis(12_000)
        );
    }

    #[test]
    fn sudden_death_with_increment() {
        let params = GoParams {
            wtime: Some(300_000),
            btime: Some(300_000),
            winc: Some(3_000),
            binc: Some(3_000),
            ..GoParams::default()
        };
        assert_eq!(
            allocate_time(&params, Color::White),
            Duration::from_millis(15_000)
        );
    }

    #[test]
    fn movestogo_with_increment() {
        let params = GoParams {
            wtime: Some(120_000),
            btime: Some(120_000),
            winc: Some(5_000),
            binc: Some(5_000),
            movestogo: Some(20),
            ..GoParams::default()
        };
        assert_eq!(
            allocate_time(&params, Color::White),
            Duration::from_millis(11_000)
        );
    }

    #[test]
    fn movestogo_without_increment() {
        let params = GoParams {
            wtime: Some(120_000),
            btime: Some(120_000),
            movestogo: Some(20),
            ..GoParams::default()
        };
        assert_eq!(
            allocate_time(&params, Color::White),
            Duration::from_millis(6_000)
        );
    }

    #[test]
    fn very_low_time() {
        let params = GoParams {
            wtime: Some(500),
            btime: Some(800),
            winc: Some(100),
            binc: Some(100),
            ..GoParams::default()
        };
        assert_eq!(
            allocate_time(&params, Color::Black),
            Duration::from_millis(132)
        );
    }

    #[test]
    fn zero_increment_no_movestogo() {
        let params = GoParams {
            wtime: Some(300_000),
            btime: Some(300_000),
            ..GoParams::default()
        };
        assert_eq!(
            allocate_time(&params, Color::Black),
            Duration::from_millis(12_000)
        );
    }
}
