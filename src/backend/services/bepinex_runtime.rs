//! The BepInEx layout and its on-disk runtime state, independent of metadata.

use crate::backend::binary::{BinaryArch, read_pe_arch};
use crate::backend::error::{AppError, AppResult};
use std::path::{Path, PathBuf};

#[cfg(not(target_os = "macos"))]
const CORECLR_FILE: &str = "coreclr.dll";
#[cfg(target_os = "macos")]
const CORECLR_FILE: &str = "libcoreclr.dylib";

pub struct BepInExRuntime<'a> {
    root: &'a Path,
}

impl<'a> BepInExRuntime<'a> {
    pub fn new(root: &'a Path) -> Self {
        Self { root }
    }

    pub fn root(&self) -> &Path {
        self.root
    }

    pub fn core_dir(&self) -> PathBuf {
        self.root.join("BepInEx/core")
    }

    pub fn assembly_path(&self) -> PathBuf {
        self.core_dir().join("BepInEx.Unity.IL2CPP.dll")
    }

    pub fn dotnet_dir(&self) -> PathBuf {
        self.root.join("dotnet")
    }

    pub fn coreclr_path(&self) -> PathBuf {
        self.dotnet_dir().join(CORECLR_FILE)
    }

    pub fn proxy_path(&self) -> PathBuf {
        self.root.join("winhttp.dll")
    }

    pub fn config_path(&self) -> PathBuf {
        self.root.join("doorstop_config.ini")
    }

    /// Inspect each time so replacing or removing files is reflected even in
    /// an existing ProfileEntry. The managed assembly alone cannot tell bitness.
    /// An installation also needs the Doorstop proxy and its configuration.
    pub fn installed_arch(&self) -> Option<BinaryArch> {
        [self.assembly_path(), self.proxy_path(), self.config_path()]
            .iter()
            .all(|path| path.is_file())
            .then(|| read_pe_arch(&self.coreclr_path()))
            .flatten()
    }

    pub fn needs_install(&self, expected: BinaryArch) -> bool {
        self.installed_arch() != Some(expected)
    }

    /// Validate extracted files rather than trusting the download URL or cache name.
    pub fn validate(&self, expected: BinaryArch) -> AppResult<()> {
        match self.installed_arch() {
            Some(actual) if actual == expected => Ok(()),
            Some(actual) => Err(AppError::validation(format!(
                "BepInEx runtime is {}, but the selected game requires {}.",
                actual.as_str(),
                expected.as_str()
            ))),
            None => Err(AppError::validation(
                "BepInEx runtime is missing, incomplete, or has an unreadable architecture.",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::test_support::{TempDir, write_test_pe, write_test_runtime};
    use std::fs;

    #[test]
    fn architecture_comes_from_native_runtime_and_tracks_file_changes() {
        let dir = TempDir::new("runtime-arch");
        let runtime = BepInExRuntime::new(&dir.0);
        write_test_runtime(&dir.0, 0x8664);
        // A managed PE may advertise x86 while running on an x64 native CLR.
        write_test_pe(&runtime.assembly_path(), 0x014c);
        write_test_pe(&runtime.coreclr_path(), 0x8664);
        assert_eq!(runtime.installed_arch(), Some(BinaryArch::X64));
        assert!(runtime.validate(BinaryArch::X64).is_ok());
        assert!(runtime.validate(BinaryArch::X86).is_err());

        write_test_pe(&runtime.coreclr_path(), 0x014c);
        assert_eq!(runtime.installed_arch(), Some(BinaryArch::X86));
        fs::remove_file(runtime.coreclr_path()).unwrap();
        assert_eq!(runtime.installed_arch(), None);
        assert!(runtime.needs_install(BinaryArch::X86));
    }

    #[test]
    fn incomplete_or_unreadable_runtime_requires_installation() {
        let dir = TempDir::new("incomplete-runtime");
        let runtime = BepInExRuntime::new(&dir.0);
        assert_eq!(runtime.installed_arch(), None);
        write_test_pe(&runtime.coreclr_path(), 0x8664);
        assert_eq!(runtime.installed_arch(), None);

        fs::create_dir_all(runtime.core_dir()).unwrap();
        fs::write(runtime.assembly_path(), b"managed").unwrap();
        fs::write(runtime.proxy_path(), b"proxy").unwrap();
        fs::write(runtime.config_path(), b"config").unwrap();
        fs::write(runtime.coreclr_path(), b"corrupt").unwrap();
        assert_eq!(runtime.installed_arch(), None);
        assert!(runtime.validate(BinaryArch::X64).is_err());

        write_test_pe(&runtime.coreclr_path(), 0xAA64);
        assert_eq!(runtime.installed_arch(), None);
    }

    #[test]
    fn missing_loader_files_require_repair() {
        let dir = TempDir::new("missing-loader");
        let runtime = BepInExRuntime::new(&dir.0);
        for file in [runtime.proxy_path(), runtime.config_path()] {
            write_test_runtime(&dir.0, 0x8664);
            assert!(!runtime.needs_install(BinaryArch::X64));
            fs::remove_file(file).unwrap();
            assert!(runtime.needs_install(BinaryArch::X64));
            assert!(runtime.validate(BinaryArch::X64).is_err());
        }
    }
}
