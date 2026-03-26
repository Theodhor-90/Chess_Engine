pub mod king;
pub mod knights;
pub mod magic;
pub mod pawns;

pub use king::generate_king_moves;
pub use knights::generate_knight_moves;
pub use pawns::generate_pawn_moves;
