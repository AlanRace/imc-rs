use std::{io, result, str::Utf8Error, string::FromUtf16Error};

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

    /// An error occurred when converting XML to UTF-16
    #[error("An error occurred when converting XML to UTF-16")]
    Utf16Erorr {
        #[from]
        /// The original error that was raised.
        source: FromUtf16Error,
    },

    /// An unknown tag appeared in the XML file
    #[error("An unknown tag appeared in the XML file")]
    UnknownTag { name: String },

    /// An error occured when parsing part of the XML file (conversion to UTF-8)
    #[error("An error occured when parsing part of the XML file (conversion to UTF-8)")]
    InvalidUtf8 {
        #[from]
        /// The original error that was raised.
        source: Utf8Error,
    },

    /// An error occured when parsing the XML file
    #[error("An error occured when parsing the XML file")]
    InvalidXML {
        #[from]
        /// The original error that was raised.
        source: quick_xml::Error,
    },
}
