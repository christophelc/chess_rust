use crate::ui::notation::{epd, san};
use std::fs::File;
use std::io::{self, BufRead};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EpdFileReaderError {
    #[error("IO error occurred: {0}")]
    Io(#[from] io::Error),

    #[error("EPD error occurred in file '{file}' on line {line}: {epd_error}")]
    Epd {
        file: String,
        #[source]
        epd_error: epd::EpdError,
        line: u64,
    },

    #[error("Unknown error occurred")]
    Unknown,
}
pub trait EpdRead {
    fn epd_read(&self, lang: &san::Lang) -> Result<Vec<epd::Epd>, EpdFileReaderError>;
}

#[derive(Debug)]
pub struct EpdErrorWithLine {
    file: String,
    epd_error: epd::EpdError,
    line: u64,
}
impl std::fmt::Display for EpdErrorWithLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "File: {} Line number: {} {}",
            self.file, self.line, self.epd_error
        )
    }
}

impl EpdErrorWithLine {
    pub fn new(file: String, epd_error: epd::EpdError, line: u64) -> Self {
        Self {
            file,
            epd_error,
            line,
        }
    }
    pub fn epd_error(&self) -> &epd::EpdError {
        &self.epd_error
    }
    pub fn line(self) -> u64 {
        self.line
    }
}

/// Represents a file containing an EPD (Extended Position Description) string.
///
/// This struct wraps a `String` to store the path of an EPD file, which is
/// commonly used in chess engines to describe chess positions.
#[derive(Debug)]
pub struct EpdFile(pub String);

impl EpdRead for EpdFile {
    fn epd_read(&self, lang: &san::Lang) -> Result<Vec<epd::Epd>, EpdFileReaderError> {
        let mut result: Vec<epd::Epd> = vec![];
        let file = File::open(&self.0)?; // May return EpdFileReaderError::Io
        let reader = io::BufReader::new(file);

        for (line_num, line) in reader.lines().enumerate() {
            let epd_str = line?; // May return EpdFileReaderError::Io
            match epd::Epd::decode(&epd_str, lang) {
                Ok(epd_value) => result.push(epd_value),
                Err(epd_error) => {
                    return Err(EpdFileReaderError::Epd {
                        file: self.0.clone(),
                        epd_error,
                        line: line_num as u64 + 1,
                    });
                }
            }
        }
        Ok(result)
    }
}
