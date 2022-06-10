use std::io;

use lz4_flex::block::DecompressError;
use thiserror::Error;

use crate::acquisition::AcquisitionIdentifier;

#[derive(Error, Debug)]

/// Describes what has gone wrong with reading an .mcd file
pub enum MCDError {
    /// An I/O error occurred
    #[error("An I/O error occured")]
    Io {
        #[from]
        /// The original error that was raised.
        source: io::Error,
        //backtrace: Backtrace,
    },
    /// Requested spectrum index is outside of expected range
    #[error("index `{index}` not in range (0..{num_spectra})")]
    InvalidIndex {
        /// The index specified
        index: usize,
        /// The number of spectra for the given acquisition
        num_spectra: usize,
    },
    /// Issue when decompressing binary data
    #[error("An error occured when decompressing: {source}")]
    Decompress {
        /// The original error that was raised
        #[from]
        source: DecompressError,
    },
    /// No channel exists which matches the specified `AcquisitionIdentifier`
    #[error("No such channel exists for this acquisition {acquisition}")]
    NoSuchChannel {
        /// `AcquisitionIdentifier` used to request a specific channel
        acquisition: AcquisitionIdentifier,
    },
}
