use crate::{
    checksum::{compute_checksum_file, ChecksumComputeError},
    ChecksumAlgorithm, Payload,
};
use digest::Digest;
use std::path::Path;
use tokio::fs;

#[derive(thiserror::Error, Debug, PartialEq)]
/// Possible errors when creating bagit containers
pub enum GenerateError {
    /// See [`ChecksumComputeError`]
    #[error("Failed to compute checksum: {0}")]
    ComputeChecksum(#[from] ChecksumComputeError),
    /// This should not be possible, but file does not have a name
    #[error("File has no name! This should not be possible")]
    FileHasNoName,
    /// Failed to create directory on filesystem
    #[error("Failed to create payload directory")]
    OpenChecksumFile(std::io::ErrorKind),
    /// Failed to read file and/or create file on filesystem
    #[error("Failed to copy file to payload directory")]
    CopyToPayloadFolder(std::io::ErrorKind),
    /// Failed to compute relative path of newly copied payload
    #[error("Failed to get relative path of file inside bag: {0}")]
    StripPrefixPath(#[from] std::path::StripPrefixError),
}

impl<'algo> super::BagIt<'_, 'algo> {
    /// Create an empty bag
    ///
    /// # Arguments
    ///
    /// * `directory` - Path where the bag will reside
    /// * `checksum_algorithm` - Algorithm used when generating manifest file
    pub fn new_empty<ChecksumAlgo: Digest>(
        directory: impl AsRef<Path>,
        checksum_algorithm: &'algo ChecksumAlgorithm<ChecksumAlgo>,
    ) -> Self {
        Self {
            path: directory.as_ref().to_path_buf(),
            checksum_algorithm: checksum_algorithm.algorithm(),
            items: vec![],
        }
    }

    /// Compute checksum of specified `file`, copy it to bag directory, add to list of items inside the bag.
    ///
    /// # Arguments
    ///
    /// * `file` - File to add to the bag, it will be copied in the path returned by [`Self::path()`]`/data`.
    pub async fn add_file<ChecksumAlgo: Digest>(
        &mut self,
        file: impl AsRef<Path>,
    ) -> Result<(), GenerateError> {
        let file_checksum = compute_checksum_file::<ChecksumAlgo>(&file).await?;

        // Create payload directory if it does not exist yet
        let mut destination = self.path.join("data/");
        fs::create_dir_all(&destination)
            .await
            .map_err(|e| GenerateError::OpenChecksumFile(e.kind()))?;

        // Construct path of file inside payload directory
        let file_name = file
            .as_ref()
            .file_name()
            .ok_or(GenerateError::FileHasNoName)?;
        destination.push(file_name);

        // Copy file
        fs::copy(file, &destination)
            .await
            .map_err(|e| GenerateError::CopyToPayloadFolder(e.kind()))?;

        let relative_path = destination.strip_prefix(&self.path)?.to_path_buf();

        // Add to list of items in bag
        self.items.push(Payload::new(relative_path, file_checksum));

        Ok(())
    }

    /// Procedure to make a bagit container ready for distribution
    ///
    /// - Write manifest file with payloads and their checksums
    /// - Bagit file declaration
    pub async fn finalize(&self) -> Result<(), std::io::Error> {
        self.write_manifest_file().await?;
        self.write_bagit_file().await?;

        Ok(())
    }

    async fn write_manifest_file(&self) -> Result<(), std::io::Error> {
        let manifest_name = format!("manifest-{}.txt", self.checksum_algorithm);
        let manifest_path = self.path.join(manifest_name);

        let contents = self
            .items
            .iter()
            .map(Payload::to_string)
            .collect::<Vec<_>>()
            .join("\n");

        fs::write(manifest_path, contents).await
    }

    async fn write_bagit_file(&self) -> Result<(), std::io::Error> {
        let manifest_path = self.path.join("bagit.txt");
        let contents = "BagIt-Version: 1.0\nTag-File-Character-Encoding: UTF-8\n";

        fs::write(manifest_path, contents).await
    }
}

#[cfg(test)]
mod test {
    use crate::{Algorithm, BagIt, ChecksumAlgorithm};
    use sha2::Sha256;

    #[tokio::test]
    async fn basic_bag_sha256() {
        let temp_directory = tempfile::Builder::new()
            .suffix("my-awesome-bag")
            .tempdir()
            .unwrap();
        let temp_directory = temp_directory.path();

        let algo = ChecksumAlgorithm::<Sha256>::new(Algorithm::Sha256);

        let mut bag = BagIt::new_empty(temp_directory, &algo);

        let mut source_directory = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        source_directory.push("tests/sample-bag/data");

        // Add files to the bag
        let temp_payload_destination = temp_directory.join("data");
        for file in [
            "bagit.md",
            "paper_bag.jpg",
            "rfc8493.txt",
            "sources.csv",
            "totebag.jpg",
        ] {
            bag.add_file::<Sha256>(source_directory.join(file))
                .await
                .unwrap();
            assert!(temp_payload_destination.join(file).is_file());
        }

        // Manifest file
        let manifest_name = format!("manifest-{}.txt", algo.algorithm());
        let manifest_file = temp_directory.join(manifest_name);
        assert!(!manifest_file.is_file());

        // Bagit file
        let bagit_file = temp_directory.join("bagit.txt");
        assert!(!bagit_file.is_file());

        // Finalize bag
        assert!(bag.finalize().await.is_ok());

        // Make sure files have been created
        assert!(manifest_file.is_file());
        assert!(bagit_file.is_file());
    }
}
