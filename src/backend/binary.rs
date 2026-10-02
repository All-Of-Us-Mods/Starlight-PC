//! PE inspection: the architecture of game executables and native runtimes,
//! and the version resource of plugin assemblies.

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

/// A binary's name and version from its Win32 version resource. .NET
/// compilers fill `FileDescription` from `[AssemblyTitle]` and
/// `ProductVersion` from `[AssemblyInformationalVersion]`; both default to the
/// assembly's name and version. `ProductName` is avoided: native libraries
/// often set it to their vendor's product, e.g. "Microsoft® Windows® Operating
/// System".
#[derive(Debug, Default, PartialEq, Eq)]
pub struct VersionInfo {
    pub name: Option<String>,
    pub version: Option<String>,
}

/// Most of a plugin is IL and embedded assets; only the headers and the
/// resource section are read. The cap bounds what a malformed header can ask for.
const MAX_RESOURCE_SECTION: u32 = 1 << 20;
const RT_VERSION: u32 = 16;

/// Read a PE file's version resource. Missing, malformed and resource-less
/// binaries have none.
pub fn read_pe_version_info(path: &Path) -> Option<VersionInfo> {
    let mut file = File::open(path).ok()?;
    let mut headers = Vec::new();
    (&mut file).take(0x1000).read_to_end(&mut headers).ok()?;
    if headers.get(..2)? != b"MZ" {
        return None;
    }
    let pe = u32_at(&headers, 0x3C)? as usize;
    if headers.get(pe..pe + 4)? != b"PE\0\0" {
        return None;
    }
    let section_count = u16_at(&headers, pe + 6)? as usize;
    let optional = pe + 24;
    let directories = match u16_at(&headers, optional)? {
        0x10b => optional + 96,
        0x20b => optional + 112,
        _ => return None,
    };
    if u32_at(&headers, directories - 4)? <= 2 {
        return None;
    }
    let resource_rva = u32_at(&headers, directories + 16)?;
    let sections = optional + u16_at(&headers, pe + 20)? as usize;
    let (base, size, offset) = (0..section_count).find_map(|ix| {
        let header = sections + ix * 40;
        let base = u32_at(&headers, header + 12)?;
        let size = u32_at(&headers, header + 16)?;
        let offset = u32_at(&headers, header + 20)?;
        (base <= resource_rva && resource_rva - base < size).then_some((base, size, offset))
    })?;
    let mut section = Vec::new();
    file.seek(SeekFrom::Start(offset.into())).ok()?;
    file.take(size.min(MAX_RESOURCE_SECTION).into())
        .read_to_end(&mut section)
        .ok()?;

    // Resource tree: type -> name -> language -> data. Assemblies carry a
    // single version resource, so the first name and language are it.
    let tree = section.get((resource_rva - base) as usize..)?;
    let names = resource_subdirectory(tree, 0, Some(RT_VERSION))?;
    let languages = resource_subdirectory(tree, names, None)?;
    let leaf = resource_child(tree, languages, None)? as usize;
    let data_start = u32_at(tree, leaf)?.checked_sub(base)? as usize;
    let data_len = u32_at(tree, leaf + 4)? as usize;
    let data = section.get(data_start..data_start.checked_add(data_len)?)?;
    parse_version_info(data)
}

/// `VS_VERSIONINFO` -> `StringFileInfo` -> string tables -> strings.
fn parse_version_info(data: &[u8]) -> Option<VersionInfo> {
    let root = VersionBlock::parse(data)?;
    if root.key != "VS_VERSION_INFO" {
        return None;
    }
    let mut info = VersionInfo::default();
    let strings = root
        .children()
        .filter(|block| block.key == "StringFileInfo")
        .flat_map(|block| block.children())
        .flat_map(|table| table.children());
    for string in strings {
        let value = utf16_until_nul(string.value);
        let value = value.trim();
        if value.is_empty() {
            continue;
        }
        match string.key.as_str() {
            "FileDescription" => {
                info.name.get_or_insert_with(|| value.to_string());
            }
            // SDK-style projects append `+<commit>` build metadata.
            "ProductVersion" => {
                let version = value.split('+').next().unwrap_or(value).trim();
                info.version.get_or_insert_with(|| version.to_string());
            }
            _ => {}
        }
    }
    Some(info)
}

/// One node of a version resource: a UTF-16 key, a value and child nodes,
/// each part aligned to 4 bytes.
struct VersionBlock<'a> {
    key: String,
    value: &'a [u8],
    children: &'a [u8],
}

impl<'a> VersionBlock<'a> {
    fn parse(data: &'a [u8]) -> Option<Self> {
        let block = data.get(..u16_at(data, 0)? as usize)?;
        let value_len = u16_at(block, 2)? as usize;
        // Text values are measured in UTF-16 units, binary ones in bytes.
        let value_len = if u16_at(block, 4)? == 1 {
            value_len * 2
        } else {
            value_len
        };
        let key_units = block
            .get(6..)?
            .chunks_exact(2)
            .position(|unit| unit == [0, 0])?;
        let key = utf16_until_nul(&block[6..]);
        let value_start = align4(6 + (key_units + 1) * 2).min(block.len());
        let value_end = (value_start + value_len).min(block.len());
        Some(Self {
            key,
            value: &block[value_start..value_end],
            children: &block[align4(value_end).min(block.len())..],
        })
    }

    fn children(&self) -> impl Iterator<Item = VersionBlock<'a>> + use<'a> {
        let mut rest = self.children;
        std::iter::from_fn(move || {
            let block = VersionBlock::parse(rest)?;
            let len = u16_at(rest, 0)? as usize;
            rest = rest.get(align4(len).max(1)..).unwrap_or_default();
            Some(block)
        })
    }
}

/// The subdirectory under the resource directory at `dir` matching `id`
/// (or the first one).
fn resource_subdirectory(tree: &[u8], dir: usize, id: Option<u32>) -> Option<usize> {
    let child = resource_child(tree, dir, id)?;
    (child & 0x8000_0000 != 0).then_some((child & 0x7FFF_FFFF) as usize)
}

/// The raw offset of the entry under the resource directory at `dir` matching
/// `id` (or the first one); its high bit marks a subdirectory.
fn resource_child(tree: &[u8], dir: usize, id: Option<u32>) -> Option<u32> {
    let count = u16_at(tree, dir + 12)? as usize + u16_at(tree, dir + 14)? as usize;
    (0..count)
        .map(|ix| dir + 16 + ix * 8)
        .find(|&entry| id.is_none_or(|id| u32_at(tree, entry) == Some(id)))
        .and_then(|entry| u32_at(tree, entry + 4))
}

fn utf16_until_nul(bytes: &[u8]) -> String {
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|unit| u16::from_le_bytes([unit[0], unit[1]]))
        .take_while(|&unit| unit != 0)
        .collect();
    String::from_utf16_lossy(&units)
}

fn align4(offset: usize) -> usize {
    (offset + 3) & !3
}

fn u16_at(bytes: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        bytes.get(offset..offset + 2)?.try_into().ok()?,
    ))
}

fn u32_at(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        bytes.get(offset..offset + 4)?.try_into().ok()?,
    ))
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

    #[test]
    fn reads_file_description_and_product_version_without_build_metadata() {
        let dir = TempDir::new("version-info");
        let path = dir.0.join("Plugin.dll");
        fs::write(
            &path,
            pe_with_version_info(&[
                ("ProductName", "Microsoft® Windows® Operating System"),
                ("FileDescription", "MiraAPI"),
                ("ProductVersion", "0.5.0+0123abcd"),
            ]),
        )
        .unwrap();
        assert_eq!(
            read_pe_version_info(&path),
            Some(VersionInfo {
                name: Some("MiraAPI".into()),
                version: Some("0.5.0".into()),
            })
        );

        fs::write(&path, pe_with_version_info(&[("FileDescription", " ")])).unwrap();
        assert_eq!(read_pe_version_info(&path), Some(VersionInfo::default()));
        // A PE without a resource section has no version info.
        fs::write(&path, pe_bytes(0x014c)).unwrap();
        assert_eq!(read_pe_version_info(&path), None);
    }

    /// A PE32 whose only section holds a version resource with `strings`.
    fn pe_with_version_info(strings: &[(&str, &str)]) -> Vec<u8> {
        let strings: Vec<u8> = strings
            .iter()
            .flat_map(|(key, value)| version_block(key, 1, &utf16z(value), &[]))
            .collect();
        let table = version_block("000004b0", 1, &[], &strings);
        let file_info = version_block("StringFileInfo", 1, &[], &table);
        let version = version_block("VS_VERSION_INFO", 0, &[0; 52], &file_info);

        let (rva, raw) = (0x1000u32, 0x200usize);
        let mut tree = vec![0u8; 88];
        for (dir, id, child) in [
            (0, 16u32, 0x8000_0018u32),
            (24, 1, 0x8000_0030),
            (48, 0x409, 72),
        ] {
            tree[dir + 14..dir + 16].copy_from_slice(&1u16.to_le_bytes());
            tree[dir + 16..dir + 20].copy_from_slice(&id.to_le_bytes());
            tree[dir + 20..dir + 24].copy_from_slice(&child.to_le_bytes());
        }
        tree[72..76].copy_from_slice(&(rva + 88).to_le_bytes());
        tree[76..80].copy_from_slice(&(version.len() as u32).to_le_bytes());
        tree.extend(version);

        let mut bytes = pe_bytes(0x014c);
        bytes.resize(raw, 0);
        let put = |bytes: &mut Vec<u8>, at: usize, value: &[u8]| {
            bytes[at..at + value.len()].copy_from_slice(value)
        };
        put(&mut bytes, 0x46, &1u16.to_le_bytes());
        put(&mut bytes, 0x54, &224u16.to_le_bytes());
        put(&mut bytes, 0x58, &0x10bu16.to_le_bytes());
        put(&mut bytes, 0x58 + 92, &16u32.to_le_bytes());
        put(&mut bytes, 0x58 + 112, &rva.to_le_bytes());
        let section = 0x58 + 224;
        put(&mut bytes, section + 12, &rva.to_le_bytes());
        put(&mut bytes, section + 16, &(tree.len() as u32).to_le_bytes());
        put(&mut bytes, section + 20, &(raw as u32).to_le_bytes());
        bytes.extend(tree);
        bytes
    }

    fn version_block(key: &str, kind: u16, value: &[u8], children: &[u8]) -> Vec<u8> {
        let value_len = if kind == 1 {
            value.len() / 2
        } else {
            value.len()
        };
        let mut block = vec![0, 0];
        block.extend((value_len as u16).to_le_bytes());
        block.extend(kind.to_le_bytes());
        block.extend(utf16z(key));
        block.resize(align4(block.len()), 0);
        block.extend(value);
        block.resize(align4(block.len()), 0);
        block.extend(children);
        let len = (block.len() as u16).to_le_bytes();
        block[..2].copy_from_slice(&len);
        block.resize(align4(block.len()), 0);
        block
    }

    fn utf16z(text: &str) -> Vec<u8> {
        text.encode_utf16()
            .chain([0])
            .flat_map(u16::to_le_bytes)
            .collect()
    }
}
