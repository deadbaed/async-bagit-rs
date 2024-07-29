use crate::error::PayloadError;
use crate::manifest::Manifest;
use crate::metadata::{Metadata, KEY_ENCODING, KEY_VERSION};
use crate::{BagIt, ChecksumAlgorithm};
use digest::Digest;
use std::path::Path;
use std::str::FromStr;
use tokio::fs;

#[derive(thiserror::Error, Debug, PartialEq)]
/// Possible errors when reading a bagit container
pub enum ReadError {
    /// Specified path is not a directory
    #[error("Path is not a directory")]
    NotDirectory,
    /// Required metadata file is not present
    #[error("Missing `bagit.txt` file")]
    MissingBagItTxt,
    /// Got wrong tag inside `bagit.txt`
    #[error("Wrong bad declaration `bagit.txt` file on key {0}")]
    BagDeclarationKey(&'static str),
    /// Wrongly formatted `bagit.txt`
    #[error("Wrong number of lines for `bagit.txt` file")]
    BagDeclarationLines,
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
            return Err(ReadError::MissingBagItTxt);
        }

        // Read whole file (it is supposed to be 2 small lines)
        let bagit_file = fs::read_to_string(path_bagit)
            .await
            .map_err(|e| ReadError::OpenFile(e.kind()))?;

        let mut bagit_file = bagit_file
            .lines()
            // Attempt to parse metadata tags, keep only successful ones
            .filter_map(|line| Metadata::from_str(line).ok());

        // Expecting first tag to be BagIt version
        match bagit_file.next() {
            Some(Metadata::BagitVersion { .. }) => (),
            _ => return Err(ReadError::BagDeclarationKey(KEY_VERSION)),
        }

        // Expecting second tag to be Encoding (utf-8)
        match bagit_file.next() {
            Some(Metadata::Encoding) => (),
            _ => return Err(ReadError::BagDeclarationKey(KEY_ENCODING)),
        }

        // Expecting no more tags
        if bagit_file.next().is_some() {
            return Err(ReadError::BagDeclarationLines);
        }

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

        // Optional if present: validate checksums from tag manifest
        if let Some(tag_manifest) =
            Manifest::find_tag_manifest(files_in_dir.as_ref(), checksum_algorithm).await?
        {
            tag_manifest
                .get_validate_payloads::<ChecksumAlgo>(bag_it_directory.as_ref())
                .await?;
        }

        Ok(BagIt {
            path: bag_it_directory.as_ref().to_path_buf(),
            items: payloads,
            checksum_algorithm: checksum_algorithm.algorithm(),
        })
    }
}

#[cfg(test)]
mod test {

    use crate::{error::ReadError, Algorithm, BagIt, Checksum, ChecksumAlgorithm, Payload};
    use md5::Md5;
    use sha2::Sha256;
    use std::path::Path;

    #[tokio::test]
    async fn basic_bag_sha256() {
        let mut bagit_directory = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        bagit_directory.push("tests/sample-bag");

        let algo = ChecksumAlgorithm::<Sha256>::new(Algorithm::Sha256);

        let bag = BagIt::read_existing(&bagit_directory, &algo).await.unwrap();

        let expected = BagIt::from_existing_items(
            bagit_directory,
            vec![
                Payload::new(
                    Path::new("data/bagit.md"),
                    Checksum::from(
                        "eccdbbade12ba878af8f2140cb00c914f427405a987de2670e5c3014faf59f8e",
                    ),
                ),
                Payload::new(
                    Path::new("data/paper_bag.jpg"),
                    Checksum::from(
                        "2b22a8fd0dc46cbdc7a67b6cf588a03a8dd6f8ea23ce0b02e921ca5d79930bb2",
                    ),
                ),
                Payload::new(
                    Path::new("data/rfc8493.txt"),
                    Checksum::from(
                        "4964147d2e6e16442d4a6dbfbe68178a8f33c3e791c06d68a8b33f51ad821537",
                    ),
                ),
                Payload::new(
                    Path::new("data/sources.csv"),
                    Checksum::from(
                        "0fe3bd6e7c36aa2c979f3330037b220c5ca88ed0eabf16622202dc0b33c44e72",
                    ),
                ),
                Payload::new(
                    Path::new("data/totebag.jpg"),
                    Checksum::from(
                        "38ff57167d746859f6383e80eb84ec0dd84de2ab1ed126ad317e73fbf502fb31",
                    ),
                ),
            ],
            algo.algorithm(),
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
