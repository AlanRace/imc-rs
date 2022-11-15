use std::{io, result};

use lz4_flex::block::DecompressError;
use thiserror::Error;

use crate::{acquisition::AcquisitionIdentifier, ChannelIdentifier};

/// A type alias for `Result<T, imc_rs::MCDError>`.
pub type Result<T> = result::Result<T, MCDError>;

/// Describes what has gone wrong with reading an .mcd file
#[derive(Error, Debug)]
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
    /// No channel exists which matches the specified `ChannelIdentifier`
    #[error("No such channel exists")]
    InvalidChannel { channel: ChannelIdentifier },
    /// No slide present in MCD file, so likely this is not a valid .mcd file.
    #[error("No slide found in MCD file - is this a valid .mcd file?")]
    NoSlidePresent,

    /// The location of the .mcd file is required to generate a .dcm file. If this is not
    /// specified either by using .from_path() or .set_location() then this error will occur.
    #[error("No location specified, so can't generate a .dcm file.")]
    LocationNotSpecified,
}
