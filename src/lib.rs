#![feature(iter_next_chunk)]
#![warn(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))] // https://stackoverflow.com/a/61417700/4809297

/*!

Rust library to create and read BagIt containers, with the [Tokio async runtime](https://docs.rs/tokio).

# Learn about BagIt

Here are some resources to get started with BagIt containers:

- The [Wikipedia article](https://en.wikipedia.org/wiki/BagIt) to get started with the format or to get a brief explanation
- The Library of Congress of the United States made [a YouTube video](https://www.youtube.com/watch?v=l3p3ao_JSfo) (in 2009) to explain what is BagIt
- The spec of the container format: [RFC 8493](https://datatracker.ietf.org/doc/html/rfc8493)

For the integrity part of BagIt, any type implementing the `Digest` trait from the [`digest`](https://docs.rs/digest) crate can be used to compute hashes.

## Load existing bag

```no_run
use async_bagit::{Algorithm, BagIt, ChecksumAlgorithm};

# #[tokio::main]
# async fn main() -> Result<(), Box<dyn std::error::Error>> {
// Specify the algorithm to use for checksums
type AlgorithmToUse = blake3::Hasher;
let algorithm = ChecksumAlgorithm::<AlgorithmToUse>::new(Algorithm::Custom("blake3"));

// Where is the bag on the filesystem?
let bag_directory = "/somewhere/where/the/bag/will/be/placed";

// Parse bagit metadata and verify checksums of payloads.
let bag = BagIt::read_existing(bag_directory, &algorithm).await.unwrap();

// This bag is complete and valid! You can get use files knowing their data is safe to use.

# Ok(())
# }
```

For more examples of what you can do with payload files once the bag has been validated, please see [`BagIt::payload_items()`].

## Create new bag and add files

```no_run
use async_bagit::{Algorithm, BagIt, ChecksumAlgorithm};

# #[tokio::main]
# async fn main() -> Result<(), Box<dyn std::error::Error>> {
// Specify the algorithm to use for checksums
type AlgorithmToUse = blake3::Hasher;
let algorithm = ChecksumAlgorithm::<AlgorithmToUse>::new(Algorithm::Custom("blake3"));

// Where the payloads and bag metadata will be placed
let bag_directory = "/somewhere/where/the/bag/will/be/placed";

// Create bag
let mut bag = BagIt::new_empty(bag_directory, &algorithm);

// Add files inside bag
for file in [
    "handbag.jpg",
    "important.pdf",
    "hit_song.mp3",
    "viral_video.mp4",
    "dank_meme.png",
] {
    bag.add_file::<AlgorithmToUse>(file).await.unwrap();
}

// Finalize bag, make it ready for distribution
bag.finalize::<AlgorithmToUse>().await.unwrap();

// The bag is ready: do whatever you want with it! Here are a few examples:
// - Copy its contents over the network
// - Burn it on a CD-ROM
// - Put it in an archive

# Ok(())
# }
```

*/

mod algorithm;
mod checksum;
mod generate;
mod manifest;
mod metadata;
mod payload;
mod read;

/// Possible errors when manipulating BagIt containers
pub mod error {
    pub use crate::checksum::ChecksumComputeError;
    pub use crate::generate::GenerateError;
    pub use crate::payload::PayloadError;
    pub use crate::read::ReadError;
}

pub use algorithm::{Algorithm, ChecksumAlgorithm};
pub use checksum::Checksum;
pub use payload::Payload;

#[derive(Debug, PartialEq)]
/// BagIt container: A set of opaque files contained within the structure defined by RFC 8493 <https://datatracker.ietf.org/doc/html/rfc8493>
///
/// This struct represents valid and complete bags opened with [`BagIt::read_existing()`],
/// or incomplete bags in the process of adding files.
///
/// See [`BagIt::new_empty()`] and [`BagIt::add_file()`].
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
    pub(crate) fn from_existing_items(
        directory: impl AsRef<std::path::Path>,
        items: Vec<Payload<'a>>,
        checksum_algorithm: &'algo Algorithm,
    ) -> Result<Self, error::ReadError> {
        Ok(Self {
            path: directory.as_ref().to_path_buf(),
            items,
            checksum_algorithm,
        })
    }

    /// Path to the folder containing the bag
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    /// Iterator over payloads inside the bag
    ///
    /// # Examples
    ///
    /// ```
    /// # use async_bagit::{Algorithm, BagIt, ChecksumAlgorithm};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let algorithm = ChecksumAlgorithm::<sha2::Sha256>::new(Algorithm::Sha256);
    /// # let mut bagit_directory = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    /// # bagit_directory.push("tests/sample-bag");
    /// // Start by getting a valid bag
    /// let bag = BagIt::read_existing(bagit_directory, &algorithm).await.unwrap();
    ///
    /// // Get the absolute paths of all payloads in this bag
    /// let absolute_paths: Vec<std::path::PathBuf> = bag
    ///     .payload_items()
    ///     .map(|payload| payload.absolute_path(&bag))
    ///     .collect();
    ///
    /// // Find a payload by its filename
    /// let my_totebag = bag
    ///     .payload_items()
    ///     .find(|payload| {
    ///         payload
    ///             .relative_path()
    ///             .file_name()
    ///             .and_then(|file_name| file_name.to_str())
    ///             == Some("totebag.jpg")
    ///     });
    /// assert!(my_totebag.is_some());
    ///
    /// // Get unique number of file extensions in the bag
    /// let number_file_extensions = bag
    ///     .payload_items()
    ///     .filter_map(|item| item.relative_path().extension())
    ///     .collect::<std::collections::HashSet<_>>()
    ///     .len();
    /// # assert_eq!(number_file_extensions, 4);
    ///
    /// # Ok(())
    /// # }
    /// ```
    pub fn payload_items(&self) -> impl Iterator<Item = &Payload> {
        self.items.iter()
    }

    fn manifest_name(&self) -> String {
        format!("manifest-{}.txt", self.checksum_algorithm)
    }

    fn tagmanifest_name(&self) -> String {
        format!("tagmanifest-{}.txt", self.checksum_algorithm)
    }
}

#[cfg(test)]
mod test {
    use crate::{Algorithm, BagIt, Checksum, ChecksumAlgorithm, Payload};
    use sha2::Sha256;
    use std::path::Path;

    #[tokio::test]
    async fn generate_and_read_basic_bag_sha256() {
        let temp_directory = async_tempfile::TempDir::new().await.unwrap();
        let temp_directory = temp_directory.to_path_buf();

        let algo = ChecksumAlgorithm::<Sha256>::new(Algorithm::Sha256);

        // Create the bag
        {
            let mut bag = BagIt::new_empty(&temp_directory, &algo);

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
            assert_eq!(bag.finalize::<Sha256>().await, Ok(()));
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
