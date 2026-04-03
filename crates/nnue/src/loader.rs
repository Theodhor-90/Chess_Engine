use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

use crate::arch::NetworkDims;
use crate::format::{architecture_hash_for, read_header, write_header, Header, FORMAT_VERSION, MAGIC};
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

    let dims = NetworkDims {
        halfkp_features: header.halfkp_features as usize,
        l1_size: header.l1_size as usize,
        l2_size: header.l2_size as usize,
        output_size: header.output_size as usize,
    };

    let expected_hash = architecture_hash_for(&dims);
    if header.arch_hash != expected_hash {
        return Err(NnueLoadError::ArchitectureMismatch {
            file_hash: header.arch_hash,
            expected_hash,
        });
    }

    let input_weights =
        read_i16_vec(&mut reader, dims.halfkp_features * dims.l1_size, "input_weights")?;
    let input_bias = read_i16_vec(&mut reader, dims.l1_size, "input_bias")?;
    let hidden1_weights = read_i8_vec(
        &mut reader,
        dims.l2_size * 2 * dims.l1_size,
        "hidden1_weights",
    )?;
    let hidden1_bias = read_i32_vec(&mut reader, dims.l2_size, "hidden1_bias")?;
    let hidden2_weights = read_i8_vec(&mut reader, dims.l2_size, "hidden2_weights")?;
    let hidden2_bias = read_i32_single(&mut reader, "hidden2_bias")?;

    Ok(Network {
        dims,
        input_weights: input_weights.into_boxed_slice(),
        input_bias: input_bias.into_boxed_slice(),
        hidden1_weights: hidden1_weights.into_boxed_slice(),
        hidden1_bias: hidden1_bias.into_boxed_slice(),
        hidden2_weights: hidden2_weights.into_boxed_slice(),
        hidden2_bias,
    })
}

pub fn write(path: &Path, network: &Network) -> Result<(), NnueLoadError> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);

    let dims = network.dims();
    let header = Header {
        version: FORMAT_VERSION,
        arch_hash: architecture_hash_for(dims),
        halfkp_features: dims.halfkp_features as u32,
        l1_size: dims.l1_size as u32,
        l2_size: dims.l2_size as u32,
        output_size: dims.output_size as u32,
    };

    write_header(&mut writer, &header)?;
    write_i16_slice(&mut writer, &network.input_weights)?;
    write_i16_slice(&mut writer, &network.input_bias)?;
    write_i8_slice(&mut writer, &network.hidden1_weights)?;
    write_i32_slice(&mut writer, &network.hidden1_bias)?;
    write_i8_slice(&mut writer, &network.hidden2_weights)?;
    writer.write_all(&network.hidden2_bias.to_le_bytes())?;
    writer.flush()?;

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
    use crate::arch::{HALFKP_FEATURES, L1_SIZE, L2_SIZE, OUTPUT_SIZE};
    use crate::format::{architecture_hash, write_header, Header, FORMAT_VERSION, MAGIC};
    use crate::inference::forward;
    use chess_types::Color;

    fn make_deterministic_network() -> Network {
        let mut net = Network::new_zeroed(NetworkDims::default_full());
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
        let net = Network::new_zeroed(NetworkDims::default_full());
        let dir = std::env::temp_dir().join("nnue_test_bad_magic");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("bad_magic.nnue");

        write(&path, &net).unwrap();

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
    fn load_dimension_mismatch_detected_via_arch_hash() {
        let net = Network::new_zeroed(NetworkDims::default_full());
        let dir = std::env::temp_dir().join("nnue_test_dim_mismatch");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("dim_mismatch.nnue");

        write(&path, &net).unwrap();

        // Patch l1_size field (offset 16-19) to a wrong value without updating arch hash
        let mut data = std::fs::read(&path).unwrap();
        let wrong_l1: u32 = 999;
        data[16..20].copy_from_slice(&wrong_l1.to_le_bytes());
        std::fs::write(&path, &data).unwrap();

        let Err(err) = load(&path) else {
            panic!("expected error");
        };
        match err {
            NnueLoadError::ArchitectureMismatch { .. } => {}
            other => panic!("expected ArchitectureMismatch, got: {other}"),
        }

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn load_truncated_file() {
        let dir = std::env::temp_dir().join("nnue_test_truncated");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("truncated.nnue");

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
        std::io::Write::flush(&mut file).unwrap();
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
        let mut net = Network::new_zeroed(NetworkDims::default_full());
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

        let mut acc = Accumulator::new(L1_SIZE);
        for i in 0..L1_SIZE {
            acc.white[i] = i as i16;
            acc.black[i] = (L1_SIZE - 1 - i) as i16;
        }

        let original_result = forward(&acc, &net, Color::White);
        let loaded_result = forward(&acc, &loaded, Color::White);
        assert_eq!(original_result, loaded_result);
        assert_eq!(loaded_result, 63);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn load_unsupported_version() {
        let net = Network::new_zeroed(NetworkDims::default_full());
        let dir = std::env::temp_dir().join("nnue_test_bad_version");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("bad_version.nnue");

        write(&path, &net).unwrap();

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

    #[test]
    fn roundtrip_non_default_dimensions() {
        let dims = NetworkDims {
            halfkp_features: 40960,
            l1_size: 128,
            l2_size: 16,
            output_size: 1,
        };
        let mut net = Network::new_zeroed(dims);
        for (i, w) in net.input_weights.iter_mut().enumerate() {
            *w = (i % 200) as i16 - 100;
        }
        for (i, b) in net.input_bias.iter_mut().enumerate() {
            *b = (i % 30) as i16 - 15;
        }
        for (i, w) in net.hidden1_weights.iter_mut().enumerate() {
            *w = (i % 100) as i8 - 50;
        }
        for (i, b) in net.hidden1_bias.iter_mut().enumerate() {
            *b = (i as i32) * 5 - 40;
        }
        for (i, w) in net.hidden2_weights.iter_mut().enumerate() {
            *w = (i % 10) as i8 - 5;
        }
        net.hidden2_bias = 17;

        let dir = std::env::temp_dir().join("nnue_test_nondefault_roundtrip");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("small.nnue");

        write(&path, &net).unwrap();
        let loaded = load(&path).unwrap();

        assert_eq!(*loaded.dims(), dims);
        assert_eq!(&*loaded.input_weights, &*net.input_weights);
        assert_eq!(&*loaded.input_bias, &*net.input_bias);
        assert_eq!(&*loaded.hidden1_weights, &*net.hidden1_weights);
        assert_eq!(&*loaded.hidden1_bias, &*net.hidden1_bias);
        assert_eq!(&*loaded.hidden2_weights, &*net.hidden2_weights);
        assert_eq!(loaded.hidden2_bias, net.hidden2_bias);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn load_corrupted_dimensions_returns_arch_mismatch() {
        let dims = NetworkDims {
            halfkp_features: 40960,
            l1_size: 128,
            l2_size: 16,
            output_size: 1,
        };
        let net = Network::new_zeroed(dims);
        let dir = std::env::temp_dir().join("nnue_test_corrupted_dims");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("corrupted.nnue");

        write(&path, &net).unwrap();

        // Corrupt l2_size (offset 20-23) without updating arch hash
        let mut data = std::fs::read(&path).unwrap();
        let original_arch_hash = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);
        let wrong_l2: u32 = 99;
        data[20..24].copy_from_slice(&wrong_l2.to_le_bytes());
        std::fs::write(&path, &data).unwrap();

        let Err(err) = load(&path) else {
            panic!("expected error");
        };
        match err {
            NnueLoadError::ArchitectureMismatch {
                file_hash,
                expected_hash,
            } => {
                assert_eq!(file_hash, original_arch_hash);
                assert_ne!(file_hash, expected_hash);
            }
            other => panic!("expected ArchitectureMismatch, got: {other}"),
        }

        std::fs::remove_dir_all(&dir).ok();
    }
}
