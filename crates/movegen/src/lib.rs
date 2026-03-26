pub mod bishops;
pub mod king;
pub mod knights;
pub mod magic;
pub mod pawns;
pub mod rooks;

pub use bishops::generate_bishop_moves;
pub use king::generate_king_moves;
pub use knights::generate_knight_moves;
pub use pawns::generate_pawn_moves;
pub use rooks::generate_rook_moves;
