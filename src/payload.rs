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
pub enum PayloadError {
    #[error("Invalid line format")]
    InvalidLine,
    #[error("Item is not inside bag")]
    NotInsideBag,
    #[error("Failed to compute checksum: {0}")]
    ComputeChecksum(#[from] ChecksumComputeError),
    #[error("Provided checksum differs from file on disk")]
    ChecksumDiffers,
}

#[derive(Debug, PartialEq)]
/// A payload is a file inside a bag
pub struct Payload<'a> {
    checksum: Checksum<'a>,

    /// Path relative to the bag directory
    relative_path: std::path::PathBuf,
}

impl Display for Payload<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.checksum, self.relative_path.display())
    }
}

impl<'a> Payload<'a> {
    pub(crate) fn new(relative_path_file: impl AsRef<Path>, checksum: Checksum<'a>) -> Self {
        Self {
            checksum,
            relative_path: relative_path_file.as_ref().to_path_buf(),
        }
    }

    pub(crate) async fn from_manifest<'manifest, 'item, ChecksumAlgo: Digest>(
        manifest_line: &'manifest str,
        base_directory: &Path,
    ) -> Result<Self, PayloadError> {
        // TODO: wait for https://github.com/rust-lang/rust/issues/98326 to stabilize
        let [checksum_from_manifest, relative_file_path] = manifest_line
            .split_whitespace()
            .next_chunk()
            .map_err(|_| PayloadError::InvalidLine)?;

        if !relative_file_path.starts_with("data/") {
            return Err(PayloadError::NotInsideBag);
        }

        let file_path = base_directory.join(relative_file_path);
        let checksum = compute_checksum_file::<ChecksumAlgo>(&file_path).await?;

        if checksum != checksum_from_manifest.into() {
            return Err(PayloadError::ChecksumDiffers);
        }

        Ok(Self {
            checksum,
            relative_path: PathBuf::from(relative_file_path),
        })
    }

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
