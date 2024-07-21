//! Run this example with the following command in a terminal:
//!
//! ```console
//! $ cargo run --example blake3_generate --features="generate" -- /tmp/mybag
//! ```

use async_bagit::{Algorithm, BagIt, ChecksumAlgorithm};

type Blake3 = blake3::Hasher;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Where to put the created bag
    let args: Vec<String> = std::env::args().collect();
    let bag_directory = args
        .get(1)
        .expect("CLI argument representing path where bag will be created");

    println!("Creating bag in `{}`", bag_directory);

    // Use the test directory of this project
    let mut source_directory = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    source_directory.push("tests/sample-bag/data");

    // Algorithm to use for checksums
    let algorithm = ChecksumAlgorithm::<Blake3>::new(Algorithm::Custom("blake3"));

    let mut bag = BagIt::new_empty(bag_directory, &algorithm);

    // Add files inside bag
    for file in [
        source_directory.join("paper_bag.jpg"),
        source_directory.join("totebag.jpg"),
    ] {
        println!("Adding file `{}` to bag", file.display());
        bag.add_file::<Blake3>(file).await?;
    }

    // Finalize bag
    bag.finalize::<Blake3>().await?;

    println!("Your new bag is available at `{}`", bag_directory);

    Ok(())
}
