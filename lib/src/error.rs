use std::io;

use lz4_flex::block::DecompressError;
use thiserror::Error;

use crate::acquisition::AcquisitionIdentifier;

#[derive(Error, Debug)]
pub enum MCDError {
    #[error("An I/O error occured")]
    Io {
        #[from]
        source: io::Error,
        //backtrace: Backtrace,
    },
    #[error("index `{index}` not in range (0..{num_spectra})")]
    InvalidIndex { index: usize, num_spectra: usize }, // InvalidIndex(requested, num_spectra)
    #[error("An error occured when decompressing: {source}")]
    Decompress {
        #[from]
        source: DecompressError,
    },
    #[error("No such channel exists for this acquisition {acquisition}")]
    NoSuchChannel { acquisition: AcquisitionIdentifier },
}
