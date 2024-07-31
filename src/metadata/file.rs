use super::{Metadata, MetadataError};
use std::path::Path;
use std::str::FromStr;
use tokio::fs;
use tokio::io::{AsyncBufReadExt, BufReader};

#[derive(Debug, PartialEq, Default)]
pub struct MetadataFile(Vec<Metadata>);

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum MetadataFileError {
    /// Metadata errors
    #[error(transparent)]
    Metadata(#[from] MetadataError),
    /// Read file error
    #[error("Failed to read file: `{0}`")]
    ReadFile(std::io::ErrorKind),
}

impl MetadataFile {
    pub async fn read(path: impl AsRef<Path>) -> Result<Self, MetadataFileError> {
        let file = fs::File::open(path.as_ref())
            .await
            .map_err(|e| MetadataFileError::ReadFile(e.kind()))?;
        let file = BufReader::new(file);
        let mut lines = file.lines();

        let mut tags = Vec::new();

        while let Some(line) = lines
            .next_line()
            .await
            .map_err(|e| MetadataFileError::ReadFile(e.kind()))?
        {
            tags.push(Metadata::from_str(&line)?);
        }

        Ok(Self(tags))
    }

    pub async fn write(&self, path: impl AsRef<Path>) -> Result<(), std::io::Error> {
        let contents = self
            .0
            .iter()
            .map(|tag| tag.to_string())
            .collect::<Vec<_>>()
            .join("\n");

        fs::write(path.as_ref(), contents).await
    }

    pub fn add(&mut self, tag: Metadata) {
        self.0.push(tag);
    }

    pub fn tags(&self) -> impl Iterator<Item = &Metadata> {
        self.0.iter()
    }
}
