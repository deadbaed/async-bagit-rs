use crate::error::PayloadError;
use crate::manifest::Manifest;
use crate::metadata::{Metadata, MetadataFile, MetadataFileError, KEY_ENCODING, KEY_VERSION};
use crate::{BagIt, ChecksumAlgorithm};
use digest::Digest;
use std::path::Path;
use tokio::fs;

#[derive(thiserror::Error, Debug, PartialEq)]
/// Possible errors when reading bag declaration file `bagit.txt`
pub enum BagDeclarationError {
    /// Required metadata file is not present
    #[error("Missing `bagit.txt` file")]
    Missing,
    /// Error when parsing file
    #[error(transparent)]
    Metadata(#[from] MetadataFileError),
    /// Got wrong tag
    #[error("Wrong tag {0}")]
    Tag(&'static str),
    /// Wrongly formatted `bagit.txt`
    #[error("Wrong number of tags for `bagit.txt` file")]
    NumberTags,
}

#[derive(thiserror::Error, Debug, PartialEq)]
/// Possible errors when reading a bagit container
pub enum ReadError {
    /// Specified path is not a directory
    #[error("Path is not a directory")]
    NotDirectory,
    /// Error related to `bagit.txt`
    #[error("Bag declaration `bagit.txt`: {0}")]
    BagDeclaration(#[from] BagDeclarationError),
    /// Error related to `bag-info.txt`
    #[error("Bag info `bag-info.txt`: {0}")]
    BagInfo(#[from] MetadataFileError),
    /// Error related to `bag-info.txt`
    #[error("Bag info incorrect Oxum: {0}")]
    BagInfoOxum(&'static str),
    /// Failed to gather list of potential checksum files
    #[error("Listing checksum files")]
    ListChecksumFiles(std::io::ErrorKind),
    /// The algorithm asked is not present in the bag
    #[error("Requested algorithm is missing")]
    NotRequestedAlgorithm,
    /// Failed to open file
    #[error("Failed to open file")]
    OpenFile(std::io::ErrorKind),
    /// Failed to read one line
    #[error("Failed to read a line in file")]
    ReadLine(std::io::ErrorKind),
    /// See [`PayloadError`]
    #[error("Failed to process a line in checksum file: {0}")]
    ProcessManifestLine(#[from] PayloadError),
}

impl<'a, 'algo> BagIt<'a, 'algo> {
    /// Read and validate a bagit container
    ///
    /// # Examples
    ///
    /// ```
    /// # use async_bagit::{Algorithm, BagIt, ChecksumAlgorithm};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// // Specify the algorithm to verify payloads
    /// let algorithm = ChecksumAlgorithm::<sha2::Sha256>::new(Algorithm::Sha256);
    ///
    /// # let mut bagit_directory = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    /// # bagit_directory.push("tests/sample-bag/");
    /// // Read what's in the bag
    /// let bag_it = BagIt::read_existing(bagit_directory, &algorithm).await.unwrap();
    /// assert_eq!(bag_it.payload_items().count(), 5);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn read_existing<ChecksumAlgo: Digest + 'algo>(
        bag_it_directory: impl AsRef<Path>,
        checksum_algorithm: &'algo ChecksumAlgorithm<ChecksumAlgo>,
    ) -> Result<BagIt<'a, 'algo>, ReadError> {
        if !bag_it_directory.as_ref().is_dir() {
            return Err(ReadError::NotDirectory);
        }

        // Read `bagit.txt`
        let path_bagit = bag_it_directory.as_ref().join("bagit.txt");
        if !path_bagit.exists() {
            return Err(ReadError::BagDeclaration(BagDeclarationError::Missing));
        }
        let bagit_file = MetadataFile::read(path_bagit)
            .await
            .map_err(|e| ReadError::BagDeclaration(e.into()))?;
        let mut bagit_file = bagit_file.tags();

        // Expecting first tag to be BagIt version
        match bagit_file.next() {
            Some(Metadata::BagitVersion { .. }) => (),
            _ => return Err(BagDeclarationError::Tag(KEY_VERSION).into()),
        }

        // Expecting second tag to be Encoding (utf-8)
        match bagit_file.next() {
            Some(Metadata::Encoding) => (),
            _ => return Err(BagDeclarationError::Tag(KEY_ENCODING).into()),
        }

        // Expecting no more tags
        if bagit_file.next().is_some() {
            return Err(BagDeclarationError::NumberTags.into());
        }

        // Get optional `bag-info.txt`
        let path_baginfo = bag_it_directory.as_ref().join("bag-info.txt");
        let bag_info = if path_baginfo.exists() {
            Some(
                MetadataFile::read(path_baginfo)
                    .await
                    .map_err(ReadError::BagInfo)?,
            )
        } else {
            None
        };

        // Get all files in directory
        let mut dir = fs::read_dir(bag_it_directory.as_ref())
            .await
            .map_err(|e| ReadError::ListChecksumFiles(e.kind()))?;
        let mut files_in_dir = Vec::new();
        while let Some(entry) = dir
            .next_entry()
            .await
            .map_err(|e| ReadError::ListChecksumFiles(e.kind()))?
        {
            let path = entry.path();
            files_in_dir.push(path);
        }

        // Get and validate payloads from manifest of requested checksum algorithm
        let payloads = Manifest::find_manifest(files_in_dir.as_ref(), checksum_algorithm)
            .await?
            .ok_or(ReadError::NotRequestedAlgorithm)?
            .get_validate_payloads::<ChecksumAlgo>(bag_it_directory.as_ref())
            .await?;

        // Optional if present: validate number of payload files and total file size
        if let Some(ref bag_info) = bag_info {
            for tag in bag_info.tags() {
                if let Metadata::PayloadOctetStreamSummary {
                    octet_count,
                    stream_count,
                } = tag
                {
                    if *stream_count != payloads.len() {
                        // Expected number of payloads does not match
                        return Err(ReadError::BagInfoOxum("stream_count"));
                    }

                    let payload_bytes_sum = payloads.iter().map(|payload| payload.bytes()).sum();
                    if *octet_count != payload_bytes_sum {
                        // Expected total bytes does not match
                        return Err(ReadError::BagInfoOxum("octet_count"));
                    }
                }
            }
        }

        // Optional if present: validate checksums from tag manifest
        if let Some(tag_manifest) =
            Manifest::find_tag_manifest(files_in_dir.as_ref(), checksum_algorithm).await?
        {
            tag_manifest
                .get_validate_payloads::<ChecksumAlgo>(bag_it_directory.as_ref())
                .await?;
        }

        // Get tags from bag info
        let tags = bag_info
            .map(|file| file.consume_tags().into_iter().collect())
            .unwrap_or_default();

        Ok(BagIt {
            path: bag_it_directory.as_ref().to_path_buf(),
            items: payloads,
            checksum_algorithm: checksum_algorithm.algorithm(),
            tags,
        })
    }
}

#[cfg(test)]
mod test {

    use crate::{
        error::ReadError, metadata::Metadata, Algorithm, BagIt, ChecksumAlgorithm, Payload,
    };
    #[cfg(feature = "date")]
    use jiff::civil::Date;
    use md5::Md5;
    use sha2::Sha256;

    #[tokio::test]
    async fn basic_bag_sha256() {
        let mut bagit_directory = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        bagit_directory.push("tests/sample-bag");

        let algo = ChecksumAlgorithm::<Sha256>::new(Algorithm::Sha256);

        let bag = BagIt::read_existing(&bagit_directory, &algo).await.unwrap();

        let expected = BagIt::from_existing_items(
            bagit_directory,
            vec![
                Payload::test_payload(
                    "data/bagit.md",
                    "eccdbbade12ba878af8f2140cb00c914f427405a987de2670e5c3014faf59f8e",
                    6302,
                ),
                Payload::test_payload(
                    "data/paper_bag.jpg",
                    "2b22a8fd0dc46cbdc7a67b6cf588a03a8dd6f8ea23ce0b02e921ca5d79930bb2",
                    19895,
                ),
                Payload::test_payload(
                    "data/rfc8493.txt",
                    "4964147d2e6e16442d4a6dbfbe68178a8f33c3e791c06d68a8b33f51ad821537",
                    48783,
                ),
                Payload::test_payload(
                    "data/sources.csv",
                    "0fe3bd6e7c36aa2c979f3330037b220c5ca88ed0eabf16622202dc0b33c44e72",
                    369,
                ),
                Payload::test_payload(
                    "data/totebag.jpg",
                    "38ff57167d746859f6383e80eb84ec0dd84de2ab1ed126ad317e73fbf502fb31",
                    10417,
                ),
            ],
            algo.algorithm(),
            vec![
                #[cfg(feature = "date")]
                Metadata::BaggingDate(Date::new(2024, 7, 11).unwrap()),
                #[cfg(not(feature = "date"))]
                Metadata::Custom {
                    key: "Bagging-Date".into(),
                    value: "2024-07-11".into(),
                },
                Metadata::PayloadOctetStreamSummary {
                    octet_count: 85766,
                    stream_count: 5,
                },
            ],
        )
        .unwrap();

        assert_eq!(bag, expected);
    }

    #[tokio::test]
    async fn basic_bag_wrong_algorithm_md5() {
        let mut bagit_directory = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        bagit_directory.push("tests/sample-bag/");

        let algo = ChecksumAlgorithm::<Md5>::new(Algorithm::Custom("md5"));

        assert_eq!(
            BagIt::read_existing(&bagit_directory, &algo).await,
            Err(ReadError::NotRequestedAlgorithm)
        );
    }
}
