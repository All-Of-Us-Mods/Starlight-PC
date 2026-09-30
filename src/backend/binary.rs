//! Architecture inspection shared by game executables and native runtimes.

use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BinaryArch {
    X86,
    X64,
}

impl BinaryArch {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::X86 => "x86",
            Self::X64 => "x64",
        }
    }
}

/// Read a native Windows binary's COFF machine field. Missing, malformed and
/// unsupported binaries have no known architecture; callers choose any fallback.
pub fn read_pe_arch(path: &Path) -> Option<BinaryArch> {
    let mut file = File::open(path).ok()?;
    let mut dos_header = [0u8; 0x40];
    file.read_exact(&mut dos_header).ok()?;
    if &dos_header[..2] != b"MZ" {
        return None;
    }
    let pe_offset = u32::from_le_bytes(dos_header[0x3C..0x40].try_into().ok()?);
    if pe_offset < dos_header.len() as u32 {
        return None;
    }
    file.seek(SeekFrom::Start(pe_offset.into())).ok()?;
    let mut pe_header = [0u8; 6];
    file.read_exact(&mut pe_header).ok()?;
    if &pe_header[..4] != b"PE\0\0" {
        return None;
    }
    match u16::from_le_bytes([pe_header[4], pe_header[5]]) {
        0x014c => Some(BinaryArch::X86),
        0x8664 => Some(BinaryArch::X64),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::test_support::{TempDir, pe_bytes, write_test_pe};
    use std::fs;

    #[test]
    fn reads_supported_machine_types_and_rejects_unknown_ones() {
        let dir = TempDir::new("binary-arch");
        let path = dir.0.join("binary.dll");
        for (machine, expected) in [
            (0x014c, Some(BinaryArch::X86)),
            (0x8664, Some(BinaryArch::X64)),
            (0xAA64, None),
        ] {
            write_test_pe(&path, machine);
            assert_eq!(read_pe_arch(&path), expected);
        }
        fs::remove_file(&path).unwrap();
        assert_eq!(read_pe_arch(&path), None);
    }

    #[test]
    fn rejects_truncated_headers_bad_signatures_and_invalid_offsets() {
        let dir = TempDir::new("malformed-binary");
        let path = dir.0.join("binary.dll");
        let valid = pe_bytes(0x8664);
        for length in [0, 2, 0x3F, 0x40, 0x45] {
            fs::write(&path, &valid[..length]).unwrap();
            assert_eq!(read_pe_arch(&path), None);
        }
        for offset in [0, 0x40] {
            let mut bytes = valid.clone();
            bytes[offset] = b'X';
            fs::write(&path, bytes).unwrap();
            assert_eq!(read_pe_arch(&path), None);
        }
        for offset in [0u32, u32::MAX] {
            let mut bytes = valid.clone();
            bytes[0x3C..0x40].copy_from_slice(&offset.to_le_bytes());
            fs::write(&path, bytes).unwrap();
            assert_eq!(read_pe_arch(&path), None);
        }
    }
}
