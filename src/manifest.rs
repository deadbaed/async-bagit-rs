use crate::ChecksumAlgorithm;
use crate::{error::ReadError, Payload};
use digest::Digest;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::{AsyncBufReadExt, BufReader};

#[derive(Debug)]
pub(crate) struct Manifest(PathBuf);

impl AsRef<Path> for Manifest {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

impl Manifest {
    pub async fn find_manifest<ChecksumAlgo: Digest>(
        files_in_directory: &[impl AsRef<Path>],
        checksum_algorithm: &ChecksumAlgorithm<ChecksumAlgo>,
    ) -> Result<Option<Self>, ReadError> {
        Self::find(files_in_directory, checksum_algorithm, "manifest-").await
    }

    pub async fn find_tag_manifest<ChecksumAlgo: Digest>(
        files_in_directory: &[impl AsRef<Path>],
        checksum_algorithm: &ChecksumAlgorithm<ChecksumAlgo>,
    ) -> Result<Option<Self>, ReadError> {
        Self::find(files_in_directory, checksum_algorithm, "tagmanifest-").await
    }

    async fn find<ChecksumAlgo: Digest>(
        files_in_directory: &[impl AsRef<Path>],
        checksum_algorithm: &ChecksumAlgorithm<ChecksumAlgo>,
        manifest_prefix: &str,
    ) -> Result<Option<Self>, ReadError> {
        // Get all potential manifests
        let manifests = files_in_directory
            .iter()
            .filter(|potential_manifest| {
                let path = potential_manifest.as_ref();

                // Item is a regular file
                path.is_file()
                    // And
                    &&
                    // Filename starts with requested prefix
                    path
                        .file_stem()
                        .and_then(|filename| filename.to_str())
                        .map(|filename| filename.starts_with(manifest_prefix))
                        .is_some_and(|does_filename_match| does_filename_match)
                    // And
                    &&
                    // File has ".txt" extension
                    path.extension().and_then(|ext| ext.to_str()) == Some("txt")
            })
            .collect::<Vec<_>>();

        // Find manifest with algorithm name
        Ok(manifests
            .into_iter()
            .find(|path| {
                path.as_ref()
                    .file_stem()
                    .and_then(|file| file.to_str())
                    .and_then(|name| name.strip_prefix(manifest_prefix))
                    == Some(checksum_algorithm.name())
            })
            .map(|path| path.as_ref().to_path_buf())
            .map(Manifest))
    }

    pub async fn get_validate_payloads<ChecksumAlgo: Digest>(
        self,
        bag_it_directory: impl AsRef<Path>,
    ) -> Result<Vec<Payload<'static>>, ReadError> {
        let checksum_file = fs::File::open(self)
            .await
            .map_err(|e| ReadError::OpenFile(e.kind()))?;
        let checksum_file = BufReader::new(checksum_file);
        let mut checksum_lines = checksum_file.lines();

        let mut items = Vec::new();

        while let Some(line) = checksum_lines
            .next_line()
            .await
            .map_err(|e| ReadError::ReadLine(e.kind()))?
        {
            let manifest_item = Payload::from_manifest::<ChecksumAlgo>(&line, &bag_it_directory)
                .await
                .map_err(ReadError::ProcessManifestLine)?;

            items.push(manifest_item);
        }

        Ok(items)
    }
}
