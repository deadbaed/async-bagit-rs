use crate::{
    checksum::{compute_checksum_file, ChecksumComputeError},
    metadata::{Metadata, MetadataFile},
    payload::{Payload, PayloadError},
    ChecksumAlgorithm,
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
    #[error("Failed to create payload directory: {0}")]
    OpenChecksumFile(std::io::ErrorKind),
    /// Failed to read file and/or create file on filesystem
    #[error("Failed to copy file to payload directory: {0}")]
    CopyToPayloadFolder(std::io::ErrorKind),
    /// Failed to compute relative path of newly copied payload
    #[error("Failed to get relative path of file inside bag: {0}")]
    StripPrefixPath(#[from] std::path::StripPrefixError),
    /// Failed to finalize bag: usually IO
    #[error("Failed to finalize bag: {0}")]
    Finalize(std::io::ErrorKind),
    /// Payload related error
    #[error(transparent)]
    Payload(#[from] PayloadError),
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
            tags: vec![],
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

        let relative_path = destination.strip_prefix(self.path())?.to_path_buf();

        // Add to list of items in bag
        self.items
            .push(Payload::new(self.path(), relative_path, file_checksum)?);

        Ok(())
    }

    #[cfg(feature = "date")]
    /// Add ISO formatted date representing date when bag was created
    pub fn add_bagging_date(&mut self, date: jiff::civil::Date) {
        self.tags.push(Metadata::BaggingDate(date));
    }

    /// Procedure to make a bagit container ready for distribution
    ///
    /// - Write manifest file with payloads and their checksums
    /// - Bagit file declaration
    /// - Information file about bag
    /// - Manifest with checksums of files that are not data payload
    pub async fn finalize<ChecksumAlgo: Digest>(&mut self) -> Result<(), GenerateError> {
        self.write_manifest_file(self.manifest_name(), self.payload_items())
            .await
            .map_err(|e| GenerateError::Finalize(e.kind()))?;

        // Write `bagit.txt`
        let mut bagit_file = MetadataFile::default();
        bagit_file.add(Metadata::BagitVersion { major: 1, minor: 0 });
        bagit_file.add(Metadata::Encoding);
        bagit_file
            .write(self.path.join("bagit.txt"))
            .await
            .map_err(|e| GenerateError::Finalize(e.kind()))?;

        // Write `bag-info.txt`
        self.tags.push(Metadata::PayloadOctetStreamSummary {
            stream_count: self.payload_items().count(),
            octet_count: self.payload_items().map(|payload| payload.bytes()).sum(),
        });
        MetadataFile::from(self.tags.clone())
            .write(self.path.join("bag-info.txt"))
            .await
            .map_err(|e| GenerateError::Finalize(e.kind()))?;

        self.write_tagmanifest_file::<ChecksumAlgo>().await?;

        Ok(())
    }

    async fn write_manifest_file(
        &self,
        filename: String,
        payloads: impl Iterator<Item = impl ToString>,
    ) -> Result<(), std::io::Error> {
        let manifest_path = self.path.join(filename);

        let contents = payloads
            .map(|payload| payload.to_string())
            .collect::<Vec<_>>()
            .join("\n");

        fs::write(manifest_path, contents).await
    }

    async fn write_tagmanifest_file<ChecksumAlgo: Digest>(&self) -> Result<(), GenerateError> {
        // Files for tag manifest
        let items = [
            "bagit.txt".into(),
            "bag-info.txt".into(),
            self.manifest_name(),
        ];

        // Compute their checksums
        let checksums_items = futures::future::join_all(
            items
                .iter()
                .map(|file| compute_checksum_file::<ChecksumAlgo>(self.path().join(file))),
        )
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;

        // Create payloads
        let payloads = items
            .iter()
            .zip(checksums_items)
            .filter_map(|(path, checksum)| Payload::new(self.path(), path, checksum).ok());

        // Write like manifest file
        self.write_manifest_file(self.tagmanifest_name(), payloads)
            .await
            .map_err(|e| GenerateError::Finalize(e.kind()))
    }
}

#[cfg(test)]
mod test {
    use crate::{Algorithm, BagIt, ChecksumAlgorithm};
    #[cfg(feature = "date")]
    use jiff::civil::Date;
    use sha2::Sha256;

    #[tokio::test]
    async fn bag_sha256() {
        let temp_directory = async_tempfile::TempDir::new().await.unwrap();
        let temp_directory = temp_directory.to_path_buf();

        let algo = ChecksumAlgorithm::<Sha256>::new(Algorithm::Sha256);

        let mut bag = BagIt::new_empty(&temp_directory, &algo);

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

        // Bag info file
        let bag_info_file = temp_directory.join("bag-info.txt");
        assert!(!bag_info_file.is_file());

        // Tag manifest file
        let tag_manifest_name = format!("tagmanifest-{}.txt", algo.algorithm());
        let tag_manifest_file = temp_directory.join(tag_manifest_name);
        assert!(!tag_manifest_file.is_file());

        // Finalize bag
        assert_eq!(bag.finalize::<Sha256>().await, Ok(()));

        // Make sure files have been created
        assert!(manifest_file.is_file());
        assert!(bagit_file.is_file());
        assert!(bag_info_file.is_file());
        assert!(tag_manifest_file.is_file());
    }

    #[tokio::test]
    #[cfg(feature = "date")]
    async fn bag_with_date() {
        use crate::metadata::Metadata;

        let temp_directory = async_tempfile::TempDir::new().await.unwrap();
        let temp_directory = temp_directory.to_path_buf();

        let algo = ChecksumAlgorithm::<Sha256>::new(Algorithm::Sha256);

        let mut bag = BagIt::new_empty(&temp_directory, &algo);

        let mut source_directory = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        source_directory.push("tests/sample-bag/data");

        // Add files to the bag
        let temp_payload_destination = temp_directory.join("data");
        for file in ["paper_bag.jpg"] {
            bag.add_file::<Sha256>(source_directory.join(file))
                .await
                .unwrap();
            assert!(temp_payload_destination.join(file).is_file());
        }

        bag.add_bagging_date(Date::new(2024, 8, 1).unwrap());

        // Finalize bag
        assert_eq!(bag.finalize::<Sha256>().await, Ok(()));

        // Read bag, make sure date is present
        let read_bag = BagIt::read_existing::<Sha256>(temp_directory, &algo)
            .await
            .unwrap();
        assert_eq!(
            read_bag.tags,
            vec![
                Metadata::BaggingDate(Date::new(2024, 8, 1).unwrap()),
                Metadata::PayloadOctetStreamSummary {
                    octet_count: 19895,
                    stream_count: 1
                }
            ]
        );
    }
}
