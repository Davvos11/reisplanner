use std::fmt::Debug;
use std::io;
use zip::result::ZipError;

#[derive(thiserror::Error, Debug)]
pub enum GtfsError {
    #[error("Failed to download GTFS data")]
    Download(#[from] DownloadError),
    #[error("Failed to parse GTFS data")]
    Parse(#[from] ParseError),
    #[error(transparent)]
    Database(#[from] rbatis::Error),
    #[error("Failed to access files")]
    IO(#[from] io::Error),
    #[error(transparent)]
    Misc(#[from] anyhow::Error),
}

#[derive(thiserror::Error, Debug)]
pub enum DownloadError{
    #[error("Failed to get GTFS response")]
    Connection(#[from] reqwest::Error),
    #[error("Failed to parse last modified date from OVapi")]
    ParseRemoteModified(#[source] anyhow::Error),
    #[error("Failed to parse last modified date of local files")]
    ParseLocalModified(#[source] anyhow::Error),
    #[error("Failed to extract the GTFS archive")]
    Unzip(#[from] ZipError),
    #[error("Failed to access the downloaded GTFS files")]
    FileSystem(#[from] io::Error)
}

#[derive(thiserror::Error, Debug)]
pub enum ParseError{
    #[error("Failed to deserialize CSV {1}")]
    Csv(#[source] csv::Error, String),
    #[error("Failed to deserialize Protobuf {1}")]
    Protobuf(#[source] protobuf::Error, String),
    #[error("Failed to parse column from database item {1:?}")]
    Database(#[source] FieldParseError, Box<dyn Debug + Send + Sync>),
    #[error("Failed to parse column from realtime item {1:?}")]
    Realtime(#[source] FieldParseError, Box<dyn Debug + Send + Sync>),
}

#[derive(thiserror::Error, Debug)]
pub enum FieldParseError {
    #[error("Failed to parse {1} into {2}")]
    Conversion(#[source] anyhow::Error, &'static str, &'static str),
    #[error("Empty value for {0}")]
    Empty(String),
}

