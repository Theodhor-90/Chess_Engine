use chess_types::PieceKind;

pub fn mirror_square(sq: u8) -> usize {
    (sq ^ 56) as usize
}

pub fn mg_table(kind: PieceKind) -> &'static [i32; 64] {
    match kind {
        PieceKind::Pawn => &MG_PAWN_TABLE,
        PieceKind::Knight => &MG_KNIGHT_TABLE,
        PieceKind::Bishop => &MG_BISHOP_TABLE,
        PieceKind::Rook => &MG_ROOK_TABLE,
        PieceKind::Queen => &MG_QUEEN_TABLE,
        PieceKind::King => &MG_KING_TABLE,
    }
}

pub fn eg_table(kind: PieceKind) -> &'static [i32; 64] {
    match kind {
        PieceKind::Pawn => &EG_PAWN_TABLE,
        PieceKind::Knight => &EG_KNIGHT_TABLE,
        PieceKind::Bishop => &EG_BISHOP_TABLE,
        PieceKind::Rook => &EG_ROOK_TABLE,
        PieceKind::Queen => &EG_QUEEN_TABLE,
        PieceKind::King => &EG_KING_TABLE,
    }
}

// PeSTO piece-square tables, from White's perspective.
// Layout: index 0 = a1, index 63 = h8 (little-endian rank-file).
// Visual: rank 1 at the top of the array, rank 8 at the bottom.

#[rustfmt::skip]
pub const MG_PAWN_TABLE: [i32; 64] = [
      0,   0,   0,   0,   0,   0,   0,   0,  // rank 1
    -35,   2, -17, -20, -12,  27,  40, -19,  // rank 2
    -24,  -2,  -1,  -7,   6,   6,  36,  -9,  // rank 3
    -25,   1,  -2,  15,  20,   9,  13, -22,  // rank 4
    -11,  16,   9,  24,  26,  15,  20, -20,  // rank 5
     -3,  10,  29,  34,  68,  59,  28, -17,  // rank 6
    101, 137,  64,  98,  71, 129,  37,  -8,  // rank 7
      0,   0,   0,   0,   0,   0,   0,   0,  // rank 8
];

#[rustfmt::skip]
pub const EG_PAWN_TABLE: [i32; 64] = [
      0,   0,   0,   0,   0,   0,   0,   0,  // rank 1
     16,  11,  11,  13,  16,   3,   5,  -4,  // rank 2
      7,  10,  -3,   4,   3,  -2,   2,  -5,  // rank 3
     16,  12,   0,  -4,  -4,  -5,   6,   2,  // rank 4
     35,  27,  16,   8,   1,   7,  20,  20,  // rank 5
     97, 103,  88,  70,  59,  56,  85,  87,  // rank 6
    181, 176, 161, 137, 150, 135, 168, 190,  // rank 7
      0,   0,   0,   0,   0,   0,   0,   0,  // rank 8
];

#[rustfmt::skip]
pub const MG_KNIGHT_TABLE: [i32; 64] = [
   -102,  -18,  -55,  -30,  -14,  -25,  -16,  -20,  // rank 1
    -26,  -50,   -9,    0,    2,   21,  -11,  -16,  // rank 2
    -20,   -6,   15,   13,   22,   20,   28,  -13,  // rank 3
    -10,    7,   19,   16,   31,   22,   24,   -5,  // rank 4
     -6,   20,   22,   56,   40,   72,   21,   25,  // rank 5
    -44,   63,   40,   68,   87,  132,   76,   47,  // rank 6
    -70,  -38,   75,   39,   26,   65,   10,  -14,  // rank 7
   -164,  -86,  -31,  -46,   64,  -94,  -12, -104,  // rank 8
];

#[rustfmt::skip]
pub const EG_KNIGHT_TABLE: [i32; 64] = [
    -26,  -48,  -20,  -12,  -19,  -15,  -47,  -61,  // rank 1
    -39,  -17,   -7,   -2,    1,  -17,  -20,  -41,  // rank 2
    -20,    0,    2,   18,   13,    0,  -17,  -19,  // rank 3
    -15,   -3,   19,   28,   19,   20,    7,  -15,  // rank 4
    -14,    6,   25,   25,   25,   14,   11,  -15,  // rank 5
    -21,  -17,   13,   12,    2,   -6,  -16,  -38,  // rank 6
    -22,   -5,  -22,    1,   -6,  -22,  -21,  -49,  // rank 7
    -55,  -35,  -10,  -25,  -28,  -24,  -60,  -96,  // rank 8
];

#[rustfmt::skip]
pub const MG_BISHOP_TABLE: [i32; 64] = [
    -30,    0,  -11,  -18,  -10,   -9,  -36,  -18,  // rank 1
      7,   18,   19,    3,   10,   24,   36,    4,  // rank 2
      3,   18,   18,   18,   17,   30,   21,   13,  // rank 3
     -3,   16,   16,   29,   37,   15,   13,    7,  // rank 4
     -1,    8,   22,   53,   40,   40,   10,    1,  // rank 5
    -13,   40,   46,   43,   38,   53,   40,    1,  // rank 6
    -23,   19,  -15,  -10,   33,   62,   21,  -44,  // rank 7
    -26,    7,  -79,  -34,  -22,  -39,   10,   -5,  // rank 8
];

#[rustfmt::skip]
pub const EG_BISHOP_TABLE: [i32; 64] = [
    -20,   -6,  -20,   -2,   -6,  -13,   -2,  -14,  // rank 1
    -11,  -15,   -4,    2,    7,   -6,  -12,  -24,  // rank 2
     -9,    0,   11,   13,   16,    6,   -4,  -12,  // rank 3
     -3,    6,   16,   22,   10,   13,    0,   -6,  // rank 4
      0,   12,   15,   12,   17,   13,    6,    5,  // rank 5
      5,   -5,    3,    2,    1,    9,    3,    7,  // rank 6
     -5,   -1,   10,   -9,    0,  -10,   -1,  -11,  // rank 7
    -11,  -18,   -8,   -5,   -4,   -6,  -14,  -21,  // rank 8
];

#[rustfmt::skip]
pub const MG_ROOK_TABLE: [i32; 64] = [
    -16,  -10,    4,   20,   19,   10,  -34,  -23,  // rank 1
    -41,  -13,  -17,   -6,    2,   14,   -3,  -68,  // rank 2
    -42,  -22,  -13,  -14,    6,    3,   -2,  -30,  // rank 3
    -33,  -23,   -9,    2,   12,   -4,    9,  -20,  // rank 4
    -21,   -8,   10,   29,   27,   38,   -5,  -17,  // rank 5
     -2,   22,   29,   39,   20,   48,   64,   19,  // rank 6
     30,   35,   61,   65,   83,   70,   29,   47,  // rank 7
     35,   45,   35,   54,   66,   12,   34,   46,  // rank 8
];

#[rustfmt::skip]
pub const EG_ROOK_TABLE: [i32; 64] = [
     -6,    5,    6,    2,   -2,  -10,    7,  -17,  // rank 1
     -3,   -3,    3,    5,   -6,   -6,   -8,    0,  // rank 2
     -1,    3,   -2,    2,   -4,   -9,   -5,  -13,  // rank 3
      6,    8,   11,    7,   -2,   -3,   -5,   -8,  // rank 4
      7,    6,   16,    4,    5,    4,    2,    5,  // rank 5
     10,   10,   10,    8,    7,    0,   -2,    0,  // rank 6
     14,   16,   16,   14,    0,    6,   11,    6,  // rank 7
     16,   13,   21,   18,   15,   15,   11,    8,  // rank 8
];

#[rustfmt::skip]
pub const MG_QUEEN_TABLE: [i32; 64] = [
      2,  -15,   -6,   13,  -12,  -22,  -28,  -47,  // rank 1
    -32,   -5,   14,    5,   11,   18,    0,    4,  // rank 2
    -11,    5,   -8,    1,   -2,    5,   17,    8,  // rank 3
     -6,  -23,   -6,   -7,    1,   -1,    6,    0,  // rank 4
    -24,  -24,  -13,  -13,    2,   20,    1,    4,  // rank 5
    -10,  -14,   10,   11,   32,   59,   50,   60,  // rank 6
    -21,  -36,   -2,    4,  -13,   60,   31,   57,  // rank 7
    -25,    3,   32,   15,   62,   47,   46,   48,  // rank 8
];

#[rustfmt::skip]
pub const EG_QUEEN_TABLE: [i32; 64] = [
    -30,  -25,  -19,  -40,   -2,  -29,  -17,  -38,  // rank 1
    -19,  -20,  -27,  -13,  -13,  -20,  -33,  -29,  // rank 2
    -13,  -24,   18,    9,   12,   20,   13,    8,  // rank 3
    -15,   31,   22,   50,   34,   37,   42,   26,  // rank 4
      6,   25,   27,   48,   60,   43,   60,   39,  // rank 5
    -17,    9,   12,   52,   50,   38,   22,   12,  // rank 6
    -14,   23,   35,   44,   61,   28,   33,    3,  // rank 7
     -6,   25,   25,   30,   30,   22,   13,   23,  // rank 8
];

#[rustfmt::skip]
pub const MG_KING_TABLE: [i32; 64] = [
    -12,   39,   15,  -51,   11,  -25,   27,   17,  // rank 1
      4,   10,   -5,  -61,  -40,  -13,   12,   11,  // rank 2
    -11,  -11,  -19,  -43,  -41,  -27,  -12,  -24,  // rank 3
    -46,    2,  -24,  -36,  -43,  -41,  -30,  -48,  // rank 4
    -14,  -17,   -9,  -24,  -27,  -22,  -11,  -33,  // rank 5
     -6,   27,    5,  -13,  -17,    9,   25,  -19,  // rank 6
     32,    2,  -17,   -4,   -5,   -1,  -35,  -26,  // rank 7
    -62,   26,   19,  -12,  -53,  -31,    5,   16,  // rank 8
];

#[rustfmt::skip]
pub const EG_KING_TABLE: [i32; 64] = [
    -50,  -31,  -18,   -8,  -25,  -11,  -21,  -40,  // rank 1
    -24,   -8,    7,   16,   17,    7,   -2,  -14,  // rank 2
    -16,    0,   14,   24,   26,   19,   10,   -6,  // rank 3
    -15,   -1,   24,   27,   30,   26,   12,   -8,  // rank 4
     -5,   25,   27,   30,   29,   36,   29,    6,  // rank 5
     13,   20,   26,   18,   23,   48,   47,   16,  // rank 6
     -9,   20,   17,   20,   20,   41,   26,   14,  // rank 7
    -71,  -32,  -15,  -15,   -8,   18,    7,  -14,  // rank 8
];
