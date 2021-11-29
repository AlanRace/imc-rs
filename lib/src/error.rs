use std::io;

use thiserror::Error;

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
}
