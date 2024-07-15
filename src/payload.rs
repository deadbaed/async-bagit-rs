use crate::{
    checksum::{compute_checksum_file, ChecksumComputeError},
    Checksum,
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
pub struct Payload<'a> {
    checksum: Checksum<'a>,

    /// Path relative to the bag directory
    file: std::path::PathBuf,
}

impl Display for Payload<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.checksum, self.file.display())
    }
}

impl<'a> Payload<'a> {
    pub(crate) fn new(relative_path_file: impl AsRef<Path>, checksum: Checksum<'a>) -> Self {
        Self {
            checksum,
            file: relative_path_file.as_ref().to_path_buf(),
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
            file: PathBuf::from(relative_file_path),
        })
    }
}

