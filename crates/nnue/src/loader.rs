use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

use crate::arch::{HALFKP_FEATURES, L1_SIZE, L2_SIZE, OUTPUT_SIZE};
use crate::format::{architecture_hash, read_header, write_header, Header, FORMAT_VERSION, MAGIC};
use crate::network::Network;

#[derive(Debug, thiserror::Error)]
pub enum NnueLoadError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid magic bytes: expected {expected:?}, got {got:?}")]
    InvalidMagic { expected: [u8; 4], got: [u8; 4] },
    #[error("unsupported format version: expected {expected}, got {got}")]
    UnsupportedVersion { expected: u32, got: u32 },
    #[error("architecture mismatch: file hash {file_hash:#010x}, expected {expected_hash:#010x}")]
    ArchitectureMismatch { file_hash: u32, expected_hash: u32 },
    #[error(
        "dimension mismatch: file declares {field}={file_value}, compiled expects {expected_value}"
    )]
    DimensionMismatch {
        field: &'static str,
        file_value: u32,
        expected_value: u32,
    },
    #[error("unexpected end of file while reading {context}")]
    UnexpectedEof { context: &'static str },
}

pub fn load(path: &Path) -> Result<Network, NnueLoadError> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    let (magic, header) = read_header(&mut reader).map_err(|e| map_eof(e, "header"))?;

    if magic != MAGIC {
        return Err(NnueLoadError::InvalidMagic {
            expected: MAGIC,
            got: magic,
        });
    }

    if header.version != FORMAT_VERSION {
        return Err(NnueLoadError::UnsupportedVersion {
            expected: FORMAT_VERSION,
            got: header.version,
        });
    }

    let expected_hash = architecture_hash();
    if header.arch_hash != expected_hash {
        return Err(NnueLoadError::ArchitectureMismatch {
            file_hash: header.arch_hash,
            expected_hash,
        });
    }

    check_dim(
        "halfkp_features",
        header.halfkp_features,
        HALFKP_FEATURES as u32,
    )?;
    check_dim("l1_size", header.l1_size, L1_SIZE as u32)?;
    check_dim("l2_size", header.l2_size, L2_SIZE as u32)?;
    check_dim("output_size", header.output_size, OUTPUT_SIZE as u32)?;

    let input_weights = read_i16_vec(&mut reader, HALFKP_FEATURES * L1_SIZE, "input_weights")?;
    let input_bias_vec = read_i16_vec(&mut reader, L1_SIZE, "input_bias")?;
    let hidden1_weights = read_i8_vec(&mut reader, L2_SIZE * 2 * L1_SIZE, "hidden1_weights")?;
    let hidden1_bias_vec = read_i32_vec(&mut reader, L2_SIZE, "hidden1_bias")?;
    let hidden2_weights_vec = read_i8_vec(&mut reader, L2_SIZE, "hidden2_weights")?;
    let hidden2_bias = read_i32_single(&mut reader, "hidden2_bias")?;

    let mut input_bias = Box::new([0i16; L1_SIZE]);
    input_bias.copy_from_slice(&input_bias_vec);

    let mut hidden1_bias = Box::new([0i32; L2_SIZE]);
    hidden1_bias.copy_from_slice(&hidden1_bias_vec);

    let mut hidden2_weights = Box::new([0i8; L2_SIZE]);
    hidden2_weights.copy_from_slice(&hidden2_weights_vec);

    Ok(Network {
        input_weights: input_weights.into_boxed_slice(),
        input_bias,
        hidden1_weights: hidden1_weights.into_boxed_slice(),
        hidden1_bias,
        hidden2_weights,
        hidden2_bias,
    })
}

pub fn write(path: &Path, network: &Network) -> Result<(), NnueLoadError> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);

    let header = Header {
        version: FORMAT_VERSION,
        arch_hash: architecture_hash(),
        halfkp_features: HALFKP_FEATURES as u32,
        l1_size: L1_SIZE as u32,
        l2_size: L2_SIZE as u32,
        output_size: OUTPUT_SIZE as u32,
    };

    write_header(&mut writer, &header)?;
    write_i16_slice(&mut writer, &network.input_weights)?;
    write_i16_slice(&mut writer, network.input_bias.as_ref())?;
    write_i8_slice(&mut writer, &network.hidden1_weights)?;
    write_i32_slice(&mut writer, network.hidden1_bias.as_ref())?;
    write_i8_slice(&mut writer, network.hidden2_weights.as_ref())?;
    writer.write_all(&network.hidden2_bias.to_le_bytes())?;
    writer.flush()?;

    Ok(())
}

fn check_dim(
    field: &'static str,
    file_value: u32,
    expected_value: u32,
) -> Result<(), NnueLoadError> {
    if file_value != expected_value {
        return Err(NnueLoadError::DimensionMismatch {
            field,
            file_value,
            expected_value,
        });
    }
    Ok(())
}

fn map_eof(e: std::io::Error, context: &'static str) -> NnueLoadError {
    if e.kind() == std::io::ErrorKind::UnexpectedEof {
        NnueLoadError::UnexpectedEof { context }
    } else {
        NnueLoadError::Io(e)
    }
}

fn read_i16_vec(
    reader: &mut impl Read,
    count: usize,
    context: &'static str,
) -> Result<Vec<i16>, NnueLoadError> {
    let mut buf = vec![0u8; count * 2];
    reader
        .read_exact(&mut buf)
        .map_err(|e| map_eof(e, context))?;
    let values = buf
        .chunks_exact(2)
        .map(|c| i16::from_le_bytes([c[0], c[1]]))
        .collect();
    Ok(values)
}

fn read_i8_vec(
    reader: &mut impl Read,
    count: usize,
    context: &'static str,
) -> Result<Vec<i8>, NnueLoadError> {
    let mut buf = vec![0u8; count];
    reader
        .read_exact(&mut buf)
        .map_err(|e| map_eof(e, context))?;
    let values = buf.into_iter().map(|b| b as i8).collect();
    Ok(values)
}

fn read_i32_vec(
    reader: &mut impl Read,
    count: usize,
    context: &'static str,
) -> Result<Vec<i32>, NnueLoadError> {
    let mut buf = vec![0u8; count * 4];
    reader
        .read_exact(&mut buf)
        .map_err(|e| map_eof(e, context))?;
    let values = buf
        .chunks_exact(4)
        .map(|c| i32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect();
    Ok(values)
}

fn read_i32_single(reader: &mut impl Read, context: &'static str) -> Result<i32, NnueLoadError> {
    let mut buf = [0u8; 4];
    reader
        .read_exact(&mut buf)
        .map_err(|e| map_eof(e, context))?;
    Ok(i32::from_le_bytes(buf))
}

fn write_i16_slice(writer: &mut impl Write, data: &[i16]) -> std::io::Result<()> {
    for &v in data {
        writer.write_all(&v.to_le_bytes())?;
    }
    Ok(())
}

fn write_i8_slice(writer: &mut impl Write, data: &[i8]) -> std::io::Result<()> {
    for &v in data {
        writer.write_all(&[v as u8])?;
    }
    Ok(())
}

fn write_i32_slice(writer: &mut impl Write, data: &[i32]) -> std::io::Result<()> {
    for &v in data {
        writer.write_all(&v.to_le_bytes())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accumulator::Accumulator;
    use crate::inference::forward;
    use chess_types::Color;

    fn make_deterministic_network() -> Network {
        let mut net = Network::new_zeroed();
        for (i, w) in net.input_weights.iter_mut().enumerate() {
            *w = (i % 256) as i16 - 128;
        }
        for (i, b) in net.input_bias.iter_mut().enumerate() {
            *b = (i % 50) as i16 - 25;
        }
        for (i, w) in net.hidden1_weights.iter_mut().enumerate() {
            *w = (i % 128) as i8 - 64;
        }
        for (i, b) in net.hidden1_bias.iter_mut().enumerate() {
            *b = (i as i32) * 10 - 160;
        }
        for (i, w) in net.hidden2_weights.iter_mut().enumerate() {
            *w = (i % 64) as i8 - 32;
        }
        net.hidden2_bias = 42;
        net
    }

    #[test]
    fn load_valid_roundtrip() {
        let original = make_deterministic_network();
        let dir = std::env::temp_dir().join("nnue_test_roundtrip");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("valid.nnue");

        write(&path, &original).unwrap();
        let loaded = load(&path).unwrap();

        assert_eq!(&*loaded.input_weights, &*original.input_weights);
        assert_eq!(&*loaded.input_bias, &*original.input_bias);
        assert_eq!(&*loaded.hidden1_weights, &*original.hidden1_weights);
        assert_eq!(&*loaded.hidden1_bias, &*original.hidden1_bias);
        assert_eq!(&*loaded.hidden2_weights, &*original.hidden2_weights);
        assert_eq!(loaded.hidden2_bias, original.hidden2_bias);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn load_invalid_magic() {
        let net = Network::new_zeroed();
        let dir = std::env::temp_dir().join("nnue_test_bad_magic");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("bad_magic.nnue");

        write(&path, &net).unwrap();

        // Overwrite magic bytes
        let mut data = std::fs::read(&path).unwrap();
        data[0..4].copy_from_slice(b"BAAD");
        std::fs::write(&path, &data).unwrap();

        let Err(err) = load(&path) else {
            panic!("expected error");
        };
        match err {
            NnueLoadError::InvalidMagic { expected, got } => {
                assert_eq!(expected, *b"NNUE");
                assert_eq!(got, *b"BAAD");
            }
            other => panic!("expected InvalidMagic, got: {other}"),
        }

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn load_dimension_mismatch() {
        let net = Network::new_zeroed();
        let dir = std::env::temp_dir().join("nnue_test_dim_mismatch");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("dim_mismatch.nnue");

        write(&path, &net).unwrap();

        // Patch l1_size field (offset 16-19) to a wrong value
        let mut data = std::fs::read(&path).unwrap();
        let wrong_l1: u32 = 999;
        data[16..20].copy_from_slice(&wrong_l1.to_le_bytes());
        // Keep the compiled architecture hash so we get past the arch_hash check
        // and reach the individual dimension checks
        std::fs::write(&path, &data).unwrap();

        let Err(err) = load(&path) else {
            panic!("expected error");
        };
        match err {
            NnueLoadError::DimensionMismatch {
                field,
                file_value,
                expected_value,
            } => {
                assert_eq!(field, "l1_size");
                assert_eq!(file_value, 999);
                assert_eq!(expected_value, L1_SIZE as u32);
            }
            other => panic!("expected DimensionMismatch, got: {other}"),
        }

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn load_truncated_file() {
        let dir = std::env::temp_dir().join("nnue_test_truncated");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("truncated.nnue");

        // Write a valid header but no weight data
        let mut file = std::fs::File::create(&path).unwrap();
        let header = Header {
            version: FORMAT_VERSION,
            arch_hash: architecture_hash(),
            halfkp_features: HALFKP_FEATURES as u32,
            l1_size: L1_SIZE as u32,
            l2_size: L2_SIZE as u32,
            output_size: OUTPUT_SIZE as u32,
        };
        write_header(&mut file, &header).unwrap();
        file.flush().unwrap();
        drop(file);

        let Err(err) = load(&path) else {
            panic!("expected error");
        };
        match err {
            NnueLoadError::UnexpectedEof { .. } | NnueLoadError::Io(_) => {}
            other => panic!("expected UnexpectedEof or Io, got: {other}"),
        }

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn load_and_forward_reference() {
        let mut net = Network::new_zeroed();
        // Use simple known weights for hand-verification
        for w in net.hidden1_weights.iter_mut() {
            *w = 1;
        }
        for w in net.hidden2_weights.iter_mut() {
            *w = 1;
        }

        let dir = std::env::temp_dir().join("nnue_test_forward");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("forward.nnue");

        write(&path, &net).unwrap();
        let loaded = load(&path).unwrap();

        let mut acc = Accumulator::new();
        for i in 0..L1_SIZE {
            acc.white[i] = i as i16;
            acc.black[i] = (L1_SIZE - 1 - i) as i16;
        }

        let original_result = forward(&acc, &net, Color::White);
        let loaded_result = forward(&acc, &loaded, Color::White);
        assert_eq!(original_result, loaded_result);
        // Cross-check with known value from inference tests
        assert_eq!(loaded_result, 63);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn load_unsupported_version() {
        let net = Network::new_zeroed();
        let dir = std::env::temp_dir().join("nnue_test_bad_version");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("bad_version.nnue");

        write(&path, &net).unwrap();

        // Patch version field (offset 4-7) to 99
        let mut data = std::fs::read(&path).unwrap();
        data[4..8].copy_from_slice(&99u32.to_le_bytes());
        std::fs::write(&path, &data).unwrap();

        let Err(err) = load(&path) else {
            panic!("expected error");
        };
        match err {
            NnueLoadError::UnsupportedVersion { expected, got } => {
                assert_eq!(expected, 1);
                assert_eq!(got, 99);
            }
            other => panic!("expected UnsupportedVersion, got: {other}"),
        }

        std::fs::remove_dir_all(&dir).ok();
    }
}
