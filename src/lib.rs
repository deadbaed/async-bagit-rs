#![feature(iter_next_chunk)]

mod algorithm;
mod checksum;
#[cfg(feature = "generate")]
mod generate;
mod payload;
mod read;

pub use algorithm::{Algorithm, ChecksumAlgorithm};
pub use checksum::{compute_checksum_file, Checksum, ChecksumComputeError};
#[cfg(feature = "generate")]
pub use generate::GenerateError;
pub use payload::{Payload, PayloadError};
pub use read::ReadError;

#[derive(Debug, PartialEq)]
pub struct BagIt<'a, 'algo> {
    /// Location of the bag
    path: std::path::PathBuf,

    /// What's in my bag
    items: Vec<Payload<'a>>,

    /// Which algorithm to use for checksums of the items
    checksum_algorithm: &'algo Algorithm,
}

impl<'a, 'algo> BagIt<'a, 'algo> {
    #[cfg(test)]
    pub fn from_existing_items(
        directory: impl AsRef<std::path::Path>,
        items: Vec<Payload<'a>>,
        checksum_algorithm: &'algo Algorithm,
    ) -> Result<Self, ReadError> {
        Ok(Self {
            path: directory.as_ref().to_path_buf(),
            items,
            checksum_algorithm,
        })
    }

    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    pub fn items(&self) -> impl Iterator<Item = &Payload> {
        self.items.iter()
    }
}

#[cfg(test)]
mod test {
    use crate::{Algorithm, BagIt, Checksum, ChecksumAlgorithm, Payload};
    use sha2::Sha256;
    use std::path::Path;

    #[tokio::test]
    #[cfg(feature = "generate")]
    async fn generate_and_read_basic_bag_sha256() {
        let temp_directory = tempfile::Builder::new()
            .suffix("in-n-out")
            .tempdir()
            .unwrap();
        let temp_directory = temp_directory.path();

        let algo = ChecksumAlgorithm::<Sha256>::new(Algorithm::Sha256);

        // Create the bag
        {
            let mut bag = BagIt::new_empty(temp_directory, &algo);

            let mut source_directory = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            source_directory.push("tests/sample-bag/data");

            // Add files to the bag
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
            }

            // Finalize bag
            bag.finalize().await.unwrap();
        }

        // Start from a blank slate to open the bag
        {
            let bag = BagIt::read_existing(&temp_directory, &algo).await.unwrap();

            let expected = BagIt::from_existing_items(
                temp_directory,
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
    }
}
