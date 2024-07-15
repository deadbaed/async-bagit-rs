use crate::{BagIt, ChecksumAlgorithm, Payload};
use digest::Digest;
use std::path::Path;
use tokio::fs;
use tokio::io::{AsyncBufReadExt, BufReader};

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum ReadError {
    #[error("Path is not a directory")]
    NotDirectory,
    #[error("There is no parent directory for this bag")]
    NoParent,
    #[error("Missing `bagit.txt` file")]
    MissingBagItTxt,
    #[error("Listing checksum files")]
    ListChecksumFiles(std::io::ErrorKind),
    #[error("Missing at least a checksum file")]
    MissingChecksumFiles,
    #[error("Requested algorithm is missing")]
    NotRequestedAlgorithm,
    #[error("Failed to open checksum file")]
    OpenChecksumFile(std::io::ErrorKind),
    #[error("Failed to read a line in checksum file")]
    ReadChecksumLine(std::io::ErrorKind),
    #[error("Failed to process a line in checksum file: {0}")]
    ProcessManifestLine(#[from] crate::error::PayloadError),
}

impl<'a, 'algo> BagIt<'a, 'algo> {
    pub async fn read_existing<ChecksumAlgo: Digest + 'algo>(
        bag_it_directory: impl AsRef<Path>,
        checksum_algorithm: &'algo ChecksumAlgorithm<ChecksumAlgo>,
    ) -> Result<BagIt<'a, 'algo>, ReadError> {
        if !bag_it_directory.as_ref().is_dir() {
            return Err(ReadError::NotDirectory);
        }

        let path_bagit = bag_it_directory.as_ref().join("bagit.txt");
        if !path_bagit.exists() {
            return Err(ReadError::MissingBagItTxt);
        }
        // TODO: parse bagit.txt

        let mut dir = fs::read_dir(bag_it_directory.as_ref())
            .await
            .map_err(|e| ReadError::ListChecksumFiles(e.kind()))?;

        let mut checksum_files = Vec::new();

        while let Some(entry) = dir
            .next_entry()
            .await
            .map_err(|e| ReadError::ListChecksumFiles(e.kind()))?
        {
            let path = entry.path();

            if
            // Item is a regular file
            path.is_file()
            // And
                &&
            // Filename starts with "manifest-"
            path
                .file_stem()
                .and_then(|filename| filename.to_str())
                .map(|filename| filename.starts_with("manifest-"))
                .is_some_and(|does_filename_match| does_filename_match)
            // And
                &&
            // File has ".txt" extension
            path.extension().and_then(|ext| ext.to_str()) == Some("txt")
            {
                checksum_files.push(path);
            }
        }

        if checksum_files.is_empty() {
            return Err(ReadError::MissingChecksumFiles);
        }

        // Get file of requested checksum
        let checksum_file = checksum_files
            .into_iter()
            .find(|path| {
                path.file_stem()
                    .and_then(|file| file.to_str())
                    .and_then(|name| name.strip_prefix("manifest-"))
                    == Some(checksum_algorithm.name())
            })
            .ok_or(ReadError::NotRequestedAlgorithm)?;
        let checksum_file = fs::File::open(checksum_file)
            .await
            .map_err(|e| ReadError::OpenChecksumFile(e.kind()))?;
        let mut checksum_file = BufReader::new(checksum_file);

        let mut items = Vec::new();

        loop {
            let mut checksum_line = String::new();
            let read_bytes = checksum_file
                .read_line(&mut checksum_line)
                .await
                .map_err(|e| ReadError::ReadChecksumLine(e.kind()))?;

            // EOF
            if read_bytes == 0 {
                break;
            }

            let manifest_item =
                Payload::from_manifest::<ChecksumAlgo>(&checksum_line, bag_it_directory.as_ref())
                    .await
                    .map_err(ReadError::ProcessManifestLine)?;

            items.push(manifest_item);
        }

        Ok(BagIt {
            path: bag_it_directory.as_ref().to_path_buf(),
            items,
            checksum_algorithm: checksum_algorithm.algorithm(),
        })
    }
}

#[cfg(test)]
mod test {

    use crate::{Algorithm, BagIt, Checksum, ChecksumAlgorithm, Payload, error::ReadError};
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
