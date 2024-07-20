//! Run this example with the following command in a terminal:
//!
//! ```console
//! $ cargo run --example read_zstd_archive -- ./tests/sample-bag.tar.zst
//! ```

use async_bagit::{Algorithm, BagIt, ChecksumAlgorithm};
use async_compression::tokio::bufread::ZstdDecoder;
use sha2::Sha256;
use tokio::{
    fs::File,
    io::{AsyncBufReadExt, BufReader},
};
use tokio_tar::Archive;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get location of archive
    let args: Vec<String> = std::env::args().collect();
    let archive_path = args
        .get(1)
        .expect("CLI argument representing path to archive contaning bag");

    // Where to put the bag
    let temp_directory = async_tempfile::TempDir::new().await.unwrap();
    let temp_directory = temp_directory.to_path_buf();

    // Open archive
    println!("Reading archive `{}`", archive_path);
    let archive_file = File::open(archive_path).await?;
    let archive_reader = BufReader::new(archive_file);

    // Decompress archive with Zstd
    let archive_decoder = ZstdDecoder::new(archive_reader);

    // Untar archive
    Archive::new(archive_decoder)
        .unpack(&temp_directory)
        .await?;

    // Algorithm to use for checksums
    let algorithm = ChecksumAlgorithm::<Sha256>::new(Algorithm::Sha256);

    // Read and list what's in the bag
    let bag_it = BagIt::read_existing(temp_directory.join("sample-bag"), &algorithm).await?;

    for payload in bag_it.payload_items() {
        println!(
            "Payload `{}` is in the bag with hash `{}`",
            payload.relative_path().display(),
            payload.checksum()
        );
    }

    // Find payload whose filename is "bagit.md"
    println!("Finding file `bagit.md` and showing its first 5 lines\n===============");
    let bagit_dot_md = bag_it
        .payload_items()
        .find(|payload| {
            payload
                .relative_path()
                .file_name()
                .and_then(|file_name| file_name.to_str())
                == Some("bagit.md")
        })
        .ok_or(Box::<dyn std::error::Error>::from(
            "failed to find payload named `bagit.md` in bag",
        ))?;

    // Read the first 5 lines of file, and display them
    let markdown_file = File::open(bagit_dot_md.absolute_path(&bag_it)).await?;
    let markdown_reader = BufReader::new(markdown_file);
    let mut lines = markdown_reader.lines();
    let mut display = String::new();
    for _ in 0..5 {
        if let Some(line) = lines.next_line().await? {
            display.push_str(&line);
            display.push('\n');
        } else {
            break;
        }
    }
    print!("{}", display);

    println!("===============\nBye!");
    Ok(())
}
