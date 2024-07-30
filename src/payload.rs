use crate::{
    checksum::{compute_checksum_file, ChecksumComputeError},
    BagIt, Checksum,
};
use digest::Digest;
use std::{
    fmt::Display,
    path::{Path, PathBuf},
};

#[derive(thiserror::Error, Debug, PartialEq)]
/// Possible errors when manipulating bagit payloads
pub enum PayloadError {
    /// Each line of manifest must be: "\<payload checksum\> \<relative path of payload\>"
    #[error("Invalid line format")]
    InvalidLine,
    /// This might happen when manifest contains wrongly formatted paths
    #[error("Failed to get absolute path")]
    Absolute(std::io::ErrorKind),
    /// Path of payload must be relative to container's path
    #[error("Payload is not inside bag")]
    NotInsideBag,
    /// See [`ChecksumComputeError`]
    #[error("Failed to compute checksum: {0}")]
    ComputeChecksum(#[from] ChecksumComputeError),
    /// Checksum is not the same after computing it and comparing with the one provided in the bag
    #[error("Provided checksum differs from file on disk")]
    ChecksumDiffers,
    /// Used for metadata tag `Oxum`
    #[error("Failed to get file size: {0}")]
    FileSize(std::io::ErrorKind),
}

#[derive(Debug, PartialEq)]
/// File inside a bagit container
pub struct Payload<'a> {
    checksum: Checksum<'a>,

    /// Path relative to the bag directory
    relative_path: std::path::PathBuf,

    /// File size in bytes
    bytes: u64,
}

impl Display for Payload<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.checksum, self.relative_path.display())
    }
}

impl<'a> Payload<'a> {
    #[cfg(test)]
    pub(crate) fn test_payload(
        relative_path_file: impl AsRef<Path>,
        checksum: &'a str,
        bytes: u64,
    ) -> Self {
        Self {
            checksum: Checksum::from(checksum),
            relative_path: PathBuf::from(relative_path_file.as_ref()),
            bytes,
        }
    }

    pub(crate) fn new(
        absolute_base_path: impl AsRef<Path>,
        relative_path_file: impl AsRef<Path>,
        checksum: Checksum<'a>,
    ) -> Result<Self, PayloadError> {
        let relative_path = relative_path_file.as_ref().to_path_buf();

        // Get absolute path
        let bytes = absolute_base_path
            .as_ref()
            .join(relative_path_file.as_ref())
            // Get file metadata
            .metadata()
            .map(|metadata| metadata.len())
            .map_err(|e| PayloadError::FileSize(e.kind()))?;

        Ok(Self {
            checksum,
            relative_path,
            bytes,
        })
    }

    pub(crate) async fn from_manifest<'manifest, 'item, ChecksumAlgo: Digest>(
        manifest_line: &'manifest str,
        base_directory: impl AsRef<Path>,
    ) -> Result<Self, PayloadError> {
        let base_directory = base_directory.as_ref();

        // TODO: wait for https://github.com/rust-lang/rust/issues/98326 to stabilize
        let [checksum_from_manifest, relative_file_path] = manifest_line
            .split_whitespace()
            .next_chunk()
            .map_err(|_| PayloadError::InvalidLine)?;

        // Absolute path of payload
        let file_path = base_directory
            .join(relative_file_path)
            .canonicalize()
            .map_err(|e| PayloadError::Absolute(e.kind()))?;

        // Get absolute path of base directory, in case there are some unresolved symlinks
        let base_directory = base_directory
            .canonicalize()
            .map_err(|e| PayloadError::Absolute(e.kind()))?;

        // Make sure payload is inside bag, prevent path traversal attacks
        if !file_path.starts_with(base_directory) {
            return Err(PayloadError::NotInsideBag);
        }

        let checksum = compute_checksum_file::<ChecksumAlgo>(&file_path).await?;

        if checksum != checksum_from_manifest.into() {
            return Err(PayloadError::ChecksumDiffers);
        }

        // File size
        let bytes = file_path
            .metadata()
            .map(|metadata| metadata.len())
            .map_err(|e| PayloadError::FileSize(e.kind()))?;

        Ok(Self {
            checksum,
            relative_path: PathBuf::from(relative_file_path),
            bytes,
        })
    }

    /// A checksum of the payload.
    ///
    /// The algorithm used is not specified, refer to either:
    /// - the moment when the payload was added
    /// - when the bag was opened.
    pub fn checksum(&self) -> &Checksum {
        &self.checksum
    }

    /// Path of payload relative to bag directory
    pub fn relative_path(&self) -> &Path {
        &self.relative_path
    }

    /// Absolute path of payload
    pub fn absolute_path(&self, bag: &BagIt) -> PathBuf {
        bag.path().join(&self.relative_path)
    }
}
