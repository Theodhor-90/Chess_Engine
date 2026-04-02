/// HalfKP feature index computation.
///
/// HalfKP is chosen over HalfKAv2 because it is the most widely adopted NNUE feature
/// set with well-understood training behavior, a smaller feature space (40,960 vs. larger
/// for HalfKAv2), and broad compatibility with existing training tooling (`bullet`,
/// `nnue-pytorch`). The reduced dimensionality means faster accumulator refresh and lower
/// memory usage, which is appropriate for an initial NNUE implementation.
use chess_types::{Color, PieceKind, Square};

use crate::arch::{HALFKP_FEATURES, NUM_COLORS, NUM_PIECE_KINDS, NUM_SQUARES};

/// Computes a unique HalfKP feature index within `0..HALFKP_FEATURES`.
///
/// The mapping is:
/// `king_sq.index() * (NUM_PIECE_KINDS * NUM_COLORS * NUM_SQUARES) + colored_piece_index * NUM_SQUARES + piece_sq.index()`
///
/// where `colored_piece_index = piece_color as usize * NUM_PIECE_KINDS + piece_kind as usize`.
///
/// # Panics (debug only)
///
/// Panics if `piece_kind` is `PieceKind::King`, since kings are not features in HalfKP.
pub fn feature_index(
    king_sq: Square,
    piece_sq: Square,
    piece_kind: PieceKind,
    piece_color: Color,
) -> usize {
    debug_assert!(
        piece_kind != PieceKind::King,
        "Kings are not features in HalfKP"
    );
    let colored_piece_index = piece_color as usize * NUM_PIECE_KINDS + piece_kind as usize;
    let idx = king_sq.index() as usize * (NUM_PIECE_KINDS * NUM_COLORS * NUM_SQUARES)
        + colored_piece_index * NUM_SQUARES
        + piece_sq.index() as usize;
    debug_assert!(idx < HALFKP_FEATURES);
    idx
}

/// A HalfKP feature descriptor.
///
/// Holds `king_sq`, `piece_sq`, `piece_kind`, and `piece_color`, and provides
/// a convenience `index()` method.
pub struct HalfKpFeature {
    /// The king square for this perspective.
    pub king_sq: Square,
    /// The square of the non-king piece.
    pub piece_sq: Square,
    /// The kind of the non-king piece.
    pub piece_kind: PieceKind,
    /// The color of the non-king piece.
    pub piece_color: Color,
}

impl HalfKpFeature {
    /// Computes the unique feature index for this feature.
    pub fn index(&self) -> usize {
        feature_index(
            self.king_sq,
            self.piece_sq,
            self.piece_kind,
            self.piece_color,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chess_types::{Color, PieceKind, Square};

    #[test]
    fn feature_index_in_range() {
        let cases = [
            (Square::E1, Square::D4, PieceKind::Knight, Color::White),
            (Square::A1, Square::H8, PieceKind::Queen, Color::Black),
            (Square::H8, Square::A1, PieceKind::Pawn, Color::White),
            (Square::D5, Square::E6, PieceKind::Bishop, Color::Black),
            (Square::G3, Square::F7, PieceKind::Rook, Color::White),
        ];
        for (king_sq, piece_sq, kind, color) in cases {
            let idx = feature_index(king_sq, piece_sq, kind, color);
            assert!(
                idx < HALFKP_FEATURES,
                "feature_index({:?}, {:?}, {:?}, {:?}) = {} >= {}",
                king_sq,
                piece_sq,
                kind,
                color,
                idx,
                HALFKP_FEATURES
            );
        }
    }

    #[test]
    fn feature_index_unique() {
        let king_sq = Square::E1;
        let piece_kinds = [
            PieceKind::Pawn,
            PieceKind::Knight,
            PieceKind::Bishop,
            PieceKind::Rook,
            PieceKind::Queen,
        ];
        let colors = [Color::White, Color::Black];
        let mut indices = Vec::new();
        for &color in &colors {
            for &kind in &piece_kinds {
                let piece_sq = Square::new(indices.len() as u8 + 8).expect("valid square");
                let idx = feature_index(king_sq, piece_sq, kind, color);
                assert!(
                    !indices.contains(&idx),
                    "Duplicate index {} for {:?} {:?} on {:?}",
                    idx,
                    color,
                    kind,
                    piece_sq
                );
                indices.push(idx);
            }
        }
        assert_eq!(indices.len(), 10);
    }

    #[test]
    fn feature_index_varies_with_king_square() {
        let piece_sq = Square::D4;
        let kind = PieceKind::Knight;
        let color = Color::White;
        let idx_e1 = feature_index(Square::E1, piece_sq, kind, color);
        let idx_d1 = feature_index(Square::D1, piece_sq, kind, color);
        assert_ne!(
            idx_e1, idx_d1,
            "Feature index should differ when king square differs"
        );
    }
}
