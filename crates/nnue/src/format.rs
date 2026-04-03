//! NNUE binary file format specification.
//!
//! Custom little-endian format for storing network weights. The format uses a
//! simple flat layout: a 28-byte header followed by weight arrays in layer order.
//!
//! ## Header layout (all little-endian)
//!
//! | Offset | Size | Field                          |
//! |--------|------|--------------------------------|
//! | 0      | 4    | Magic bytes `b"NNUE"`          |
//! | 4      | 4    | `u32` format version           |
//! | 8      | 4    | `u32` architecture hash        |
//! | 12     | 4    | `u32` halfkp_features (input)  |
//! | 16     | 4    | `u32` l1_size                  |
//! | 20     | 4    | `u32` l2_size                  |
//! | 24     | 4    | `u32` output_size              |
//!
//! ## Weight data (immediately after header)
//!
//! 1. Input weights: `HALFKP_FEATURES * L1_SIZE` × `i16` (little-endian)
//! 2. Input bias: `L1_SIZE` × `i16`
//! 3. Hidden1 weights: `L2_SIZE * 2 * L1_SIZE` × `i8`
//! 4. Hidden1 bias: `L2_SIZE` × `i32` (little-endian)
//! 5. Hidden2 weights: `L2_SIZE` × `i8`
//! 6. Hidden2 bias: 1 × `i32` (little-endian)
//!
//! ## Design rationale
//!
//! A custom format is used instead of Stockfish-compatible because our topology
//! (HalfKP 40960→256→32→1) differs from Stockfish's HalfKAv2 architecture, and
//! their nested section header scheme is tightly coupled to their specific layout.

use crate::arch::{HALFKP_FEATURES, L1_SIZE, L2_SIZE, OUTPUT_SIZE};
use std::io::{Read, Write};

pub const MAGIC: [u8; 4] = *b"NNUE";
pub const FORMAT_VERSION: u32 = 1;
pub const HEADER_SIZE: usize = 28;

pub struct Header {
    pub version: u32,
    pub arch_hash: u32,
    pub halfkp_features: u32,
    pub l1_size: u32,
    pub l2_size: u32,
    pub output_size: u32,
}

pub fn architecture_hash() -> u32 {
    let dims = [
        HALFKP_FEATURES as u32,
        L1_SIZE as u32,
        L2_SIZE as u32,
        OUTPUT_SIZE as u32,
    ];
    let mut hash: u32 = 0;
    for d in dims {
        hash ^= d;
        hash = hash.rotate_left(7);
    }
    hash
}

pub fn write_header(writer: &mut impl Write, header: &Header) -> std::io::Result<()> {
    writer.write_all(&MAGIC)?;
    writer.write_all(&header.version.to_le_bytes())?;
    writer.write_all(&header.arch_hash.to_le_bytes())?;
    writer.write_all(&header.halfkp_features.to_le_bytes())?;
    writer.write_all(&header.l1_size.to_le_bytes())?;
    writer.write_all(&header.l2_size.to_le_bytes())?;
    writer.write_all(&header.output_size.to_le_bytes())?;
    Ok(())
}

pub fn read_header(reader: &mut impl Read) -> std::io::Result<([u8; 4], Header)> {
    let mut buf4 = [0u8; 4];

    reader.read_exact(&mut buf4)?;
    let magic = buf4;

    reader.read_exact(&mut buf4)?;
    let version = u32::from_le_bytes(buf4);

    reader.read_exact(&mut buf4)?;
    let arch_hash = u32::from_le_bytes(buf4);

    reader.read_exact(&mut buf4)?;
    let halfkp_features = u32::from_le_bytes(buf4);

    reader.read_exact(&mut buf4)?;
    let l1_size = u32::from_le_bytes(buf4);

    reader.read_exact(&mut buf4)?;
    let l2_size = u32::from_le_bytes(buf4);

    reader.read_exact(&mut buf4)?;
    let output_size = u32::from_le_bytes(buf4);

    Ok((
        magic,
        Header {
            version,
            arch_hash,
            halfkp_features,
            l1_size,
            l2_size,
            output_size,
        },
    ))
}
