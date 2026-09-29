use std::fs;
use std::path::{Path, PathBuf};

pub struct TempDir(pub PathBuf);

impl TempDir {
    pub fn new(tag: &str) -> Self {
        let path = std::env::temp_dir().join(format!("starlight-{tag}-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub fn pe_bytes(machine: u16) -> Vec<u8> {
    let mut bytes = vec![0u8; 0x80];
    bytes[..2].copy_from_slice(b"MZ");
    bytes[0x3C..0x40].copy_from_slice(&0x40u32.to_le_bytes());
    bytes[0x40..0x44].copy_from_slice(b"PE\0\0");
    bytes[0x44..0x46].copy_from_slice(&machine.to_le_bytes());
    bytes
}

pub fn write_test_pe(path: &Path, machine: u16) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, pe_bytes(machine)).unwrap();
}
