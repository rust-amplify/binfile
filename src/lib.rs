// SPDX-License-Identifier: Apache-2.0

#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![deny(missing_docs)]

//! Binary files with magic numbers and versioning.
//!
//! See [`BinFile`] for the details.

#[macro_use]
extern crate amplify;

use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::ops::{Deref, DerefMut};
use std::path::Path;

/// Binary file which ensures it always starts with a given magic byte octet.
///
/// Works as a drop-in replacement for [`File`], which is checking that the file always start with
/// an octet of some specific magic number and 16-bit version.
///
/// The magic byte octet is provided as a generic constant parameter in a form of 64-bit unsigned
/// integer, which is serialized in big-endian order.
///
/// The version is provided as a generic constant parameter in a form of 16-bit unsigned integer,
/// which is serialized in big-endian order.
///
/// It is recommended to start the version numbering from 1, and not zero.
///
/// # Example
///
/// ```
/// use binfile::BinFile;
///
/// const MY_MAGIC: u64 = u64::from_be_bytes(*b"MYMAGIC!");
///
/// BinFile::<MY_MAGIC, 1>::create("target/test").unwrap();
/// ```
#[derive(Debug)]
pub struct BinFile<const MAGIC: u64, const VERSION: u16 = 1>(File);

impl<const MAGIC: u64, const VERSION: u16> Deref for BinFile<MAGIC, VERSION> {
    type Target = File;

    fn deref(&self) -> &Self::Target { &self.0 }
}

impl<const MAGIC: u64, const VERSION: u16> DerefMut for BinFile<MAGIC, VERSION> {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

impl<const MAGIC: u64, const VERSION: u16> Read for BinFile<MAGIC, VERSION> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> { self.0.read(buf) }
}

impl<const MAGIC: u64, const VERSION: u16> Write for BinFile<MAGIC, VERSION> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> { self.0.write(buf) }
    fn flush(&mut self) -> io::Result<()> { self.0.flush() }
}

impl<const MAGIC: u64, const VERSION: u16> BinFile<MAGIC, VERSION> {
    /// The magical byte octet, taken from the generic parameter of the type. It must be a big
    /// endian-serialized octet.
    pub const MAGIC: u64 = MAGIC;

    /// The version number, taken from the generic parameter of the type.
    pub const VERSION: u16 = VERSION;

    /// Opens the file in read-write mode, the same way as [`File::create`] does.
    ///
    /// Creates the file if it doesn't exist, and truncates if it does. In both cases, it writes
    /// the magic number and the version (10 bytes in total) at the start of the file. The produced
    /// file stream will start at byte offset 10.
    pub fn create(path: impl AsRef<Path>) -> io::Result<Self> {
        let mut file = File::create(path)?;
        file.write_all(&MAGIC.to_be_bytes())?;
        file.write_all(&VERSION.to_be_bytes())?;
        Ok(Self(file))
    }

    /// Creates a new file in read-write mode; error if the file exists, the same way as
    /// [`File::create_new`] does.
    ///
    /// Writes the magic number and the version (10 bytes in total) at the start of the file. The
    /// produced file stream will start at byte offset 10.
    pub fn create_new(path: impl AsRef<Path>) -> io::Result<Self> {
        let mut file = File::create_new(path)?;
        file.write_all(&MAGIC.to_be_bytes())?;
        file.write_all(&VERSION.to_be_bytes())?;
        Ok(Self(file))
    }

    /// Attempts to open a file in read-only mode the same way as [`File::open`] does.
    ///
    /// Then it reads first 10 bytes of the file and verifies them to match the magic number (8
    /// bytes) and version number (2 bytes). The produced file stream will start at byte offset 10.
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();
        let mut file = Self(File::open(&path)?);
        file.check(&path)?;
        Ok(file)
    }

    /// Attempts to open a file in read-write mode the same way as [`OpenOptions::new`], followed by
    /// `read(true)` and `write(true)` calls does.
    ///
    /// Then it reads first 10 bytes of the file and verifies them to match the magic number (8
    /// bytes) and version number (2 bytes). The produced file stream will start at byte offset 10.
    pub fn open_rw(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();
        let mut file = Self(OpenOptions::new().read(true).write(true).open(&path)?);
        file.check(&path)?;
        Ok(file)
    }

    fn check(&mut self, filename: &Path) -> io::Result<()> {
        let mut magic = [0u8; 8];
        self.read_exact(&mut magic)?;
        if magic != MAGIC.to_be_bytes() {
            return Err(io::Error::other(BinFileError::InvalidMagic {
                filename: filename.to_string_lossy().to_string(),
                expected: MAGIC,
                actual: u64::from_be_bytes(magic),
            }));
        }
        let mut version = [0u8; 2];
        self.read_exact(&mut version)?;
        if version != VERSION.to_be_bytes() {
            return Err(io::Error::other(BinFileError::InvalidVersion {
                filename: filename.to_string_lossy().to_string(),
                expected: VERSION,
                actual: u16::from_be_bytes(version),
            }));
        }
        Ok(())
    }
}

/// Errors specific to [`BinFile`], which appear as a custom errors inside [`io::Error`].
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug, Display, Error)]
#[display(doc_comments)]
pub enum BinFileError {
    /// invalid magic number {actual:#10x} instead of {expected:#10x} in file '{filename}'.
    InvalidMagic {
        #[allow(missing_docs)]
        filename: String,
        #[allow(missing_docs)]
        expected: u64,
        #[allow(missing_docs)]
        actual: u64,
    },
    /// invalid version {actual} instead of {expected} in file '{filename}'.
    InvalidVersion {
        #[allow(missing_docs)]
        filename: String,
        #[allow(missing_docs)]
        expected: u16,
        #[allow(missing_docs)]
        actual: u16,
    },
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn create() {
        const MY_MAGIC: u64 = u64::from_be_bytes(*b"MYMAGIC!");
        let mut file = BinFile::<MY_MAGIC, 1>::create("target/test1").unwrap();
        file.write_all(b"hello world").unwrap();
        file.flush().unwrap();

        let check = fs::read("target/test1").unwrap();
        assert_eq!(check, b"MYMAGIC!\x00\x01hello world");
    }

    #[test]
    fn create_new() {
        const MY_MAGIC: u64 = u64::from_be_bytes(*b"MYMAGIC!");
        fs::remove_file("target/test2").ok();
        let mut file = BinFile::<MY_MAGIC, 1>::create_new("target/test2").unwrap();
        file.write_all(b"hello world").unwrap();
        file.flush().unwrap();

        let fail = BinFile::<MY_MAGIC, 1>::create_new("target/test2");
        assert_eq!(fail.unwrap_err().kind(), io::ErrorKind::AlreadyExists);

        let check = fs::read("target/test2").unwrap();
        assert_eq!(check, b"MYMAGIC!\x00\x01hello world");
    }

    #[test]
    fn open_ro() {
        const MY_MAGIC: u64 = u64::from_be_bytes(*b"MYMAGIC!");
        let mut file = BinFile::<MY_MAGIC, 1>::create("target/test3").unwrap();
        file.write_all(b"hello world").unwrap();
        file.flush().unwrap();

        let mut file = BinFile::<MY_MAGIC, 1>::open("target/test3").unwrap();
        let mut buf = Vec::new();
        let check = file.read_to_end(&mut buf).unwrap();
        assert_eq!(check, 11);
        assert_eq!(buf, b"hello world");
    }

    #[test]
    fn open_rw() {
        const MY_MAGIC: u64 = u64::from_be_bytes(*b"MYMAGIC!");
        let mut file = BinFile::<MY_MAGIC, 1>::create("target/test5").unwrap();
        file.write_all(b"hello world").unwrap();
        file.flush().unwrap();

        let mut file = BinFile::<MY_MAGIC, 1>::open_rw("target/test5").unwrap();
        let mut buf = Vec::new();
        let check = file.read_to_end(&mut buf).unwrap();
        assert_eq!(check, 11);
        assert_eq!(buf, b"hello world");
        file.write_all(b"\nand hello again").unwrap();
    }

    #[test]
    fn open_wrong_magic() {
        const MY_MAGIC: u64 = u64::from_be_bytes(*b"MYMAGIC!");
        let mut file = BinFile::<MY_MAGIC, 1>::create("target/test4").unwrap();
        file.write_all(b"hello world").unwrap();
        file.flush().unwrap();

        let err = BinFile::<0xFFFFFF_FFFFFF, 1>::open("target/test4").unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::Other);
        assert_eq!(err.downcast::<BinFileError>().unwrap(), BinFileError::InvalidMagic {
            filename: "target/test4".to_string(),
            expected: 0xFFFFFF_FFFFFF,
            actual: MY_MAGIC,
        });
    }

    #[test]
    fn open_wrong_version() {
        const MY_MAGIC: u64 = u64::from_be_bytes(*b"MYMAGIC!");
        let mut file = BinFile::<MY_MAGIC, 1>::create("target/test5").unwrap();
        file.write_all(b"hello world").unwrap();
        file.flush().unwrap();

        let err = BinFile::<MY_MAGIC, 0x0100>::open("target/test5").unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::Other);
        assert_eq!(err.downcast::<BinFileError>().unwrap(), BinFileError::InvalidVersion {
            filename: "target/test5".to_string(),
            expected: 0x0100,
            actual: 1,
        });
    }
}
