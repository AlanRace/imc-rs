#![warn(missing_docs)]
#![warn(rustdoc::missing_doc_code_examples)]

//! This library provides a means of accessing imaging mass cytometry (Fluidigm) data stored in the (*.mcd) format.
//!
//! # Example
//!
//! ```no_run
//! extern crate imc_rs;
//!
//! use imc_rs::MCD;
//! use std::io::BufReader;
//! use std::fs::File;
//!
//! fn main() {
//!     let filename = "/location/to/data.mcd";
//!     let file = BufReader::new(File::open(filename).unwrap());
//!     let mcd = MCD::parse_with_dcm(file, filename);
//!
//!     
//! }
//! ```

mod convert;
mod error;
pub(crate) mod mcd;
mod transform;

mod acquisition;
mod calibration;
mod channel;
mod images;
mod panorama;
mod slide;

/// Provides methods for reading in cell segmentation data from Halo
pub mod halo;

pub use self::acquisition::Acquisition;
pub use self::channel::{AcquisitionChannel, ChannelIdentifier};
pub use self::panorama::Panorama;
pub use self::slide::Slide;

use std::convert::TryInto;
use std::fmt;
use std::io::{Read, Seek, SeekFrom, Write};

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use std::collections::HashMap;

use acquisition::AcquisitionIdentifier;
use calibration::{Calibration, CalibrationChannel, CalibrationFinal, CalibrationParams};
use mcd::{MCDParser, ParserState};

use image::{ImageFormat, RgbaImage};
use slide::{SlideFiducialMarks, SlideProfile};
use transform::AffineTransform;

const BUF_SIZE: usize = 4096;

/// Print to `writer` trait
pub trait Print {
    /// Formats and prints to `writer`
    fn print<W: fmt::Write + ?Sized>(&self, writer: &mut W, indent: usize) -> fmt::Result;
}

/// Represents property of having an optical image
pub trait OpticalImage {
    /// Returns whether an optical image is present
    fn has_image(&self) -> bool;
    /// Returns the binary data for the image, exactly as stored in the .mcd file
    fn image_data(&self) -> Result<Vec<u8>, std::io::Error>;
    /// Returns the format of the stored optical image
    fn image_format(&self) -> ImageFormat;
    /// Returns a decoded RgbaImage of the panorama image
    fn image(&self) -> RgbaImage;
}

/// Represents an image which is acquired on a slide
pub trait OnSlide {
    /// Returns the bounding box encompasing the image area on the slide (in μm)
    fn slide_bounding_box(&self) -> BoundingBox<f64>;
    /// Returns the affine transformation from pixel coordinates within the image to to the slide coordinates (μm)
    fn to_slide_transform(&self) -> AffineTransform<f64>;
}

/// Represents a imaging mass cytometry (*.mcd) file.
#[derive(Debug)]
pub struct MCD<T: Seek + Read> {
    reader: Arc<Mutex<T>>,
    location: String,

    xmlns: Option<String>,

    slides: HashMap<u16, Slide<T>>,
    //acquisition_order: Vec<String>,
    //acquisitions: HashMap<String, Arc<Acquisition>>,
    //acquisition_rois: Vec<AcquisitionROI>,
    //roi_points: Vec<ROIPoint>,
    calibration_finals: HashMap<u16, CalibrationFinal>,
    calibration_params: HashMap<u16, CalibrationParams>,
    calibration_channels: HashMap<u16, CalibrationChannel>,
    calibrations: HashMap<u16, Calibration>,
    slide_fiducal_marks: HashMap<u16, SlideFiducialMarks>,
    slide_profiles: HashMap<u16, SlideProfile>,
}

fn find_mcd_start(chunk: &[u8], chunk_size: usize) -> usize {
    for start_index in 0..chunk_size {
        if let Ok(_data) = std::str::from_utf8(&chunk[start_index..]) {
            return start_index - 1;
        }
    }

    0
}

fn u16_from_u8(a: &mut [u16], v: &[u8]) {
    for i in 0..a.len() {
        a[i] = (v[i * 2] as u16) | ((v[i * 2 + 1] as u16) << 8)
    }
}

impl<T: Seek + Read> MCD<T> {
    fn new(reader: T, location: &str) -> Self {
        MCD {
            reader: Arc::new(Mutex::new(reader)),
            location: location.to_owned(),
            xmlns: None,
            slides: HashMap::new(),
            //panoramas: HashMap::new(),
            //acquisition_channels: Vec::new(),
            //acquisition_order: Vec::new(),
            //acquisitions: HashMap::new(),
            //acquisition_rois: Vec::new(),
            calibration_finals: HashMap::new(),
            calibration_params: HashMap::new(),
            calibration_channels: HashMap::new(),
            calibrations: HashMap::new(),
            slide_fiducal_marks: HashMap::new(),
            slide_profiles: HashMap::new(),
        }
    }

    pub(crate) fn dcm_file(&self) -> PathBuf {
        let mut path = PathBuf::from(&self.location);
        path.set_extension("dcm");

        path
    }

    pub(crate) fn reader(&self) -> &Arc<Mutex<T>> {
        &self.reader
    }

    /// Returns a copy of the slide IDs, sorted by ID number
    pub fn slide_ids(&self) -> Vec<u16> {
        let mut ids: Vec<u16> = Vec::with_capacity(self.slides.len());

        for id in self.slides.keys() {
            ids.push(*id);
        }

        // TODO: We could just sort the slides once and then return references to the held vectors to avoid allocating
        // new ones in `pub fn slides(&self)`
        ids.sort_unstable();

        ids
    }

    /// Returns slide with a given ID number, or `None` if no such slide exists
    pub fn slide(&self, id: u16) -> Option<&Slide<T>> {
        self.slides.get(&id)
    }

    /// Returns a vector of references to slides sorted by ID number. This allocates a new vector on each call.
    pub fn slides(&self) -> Vec<&Slide<T>> {
        let mut slides = Vec::new();

        for id in self.slide_ids() {
            slides.push(
                self.slide(id)
                    .expect("Should only be finding slides that exist"),
            );
        }

        slides
    }

    fn slides_mut(&mut self) -> &mut HashMap<u16, Slide<T>> {
        &mut self.slides
    }

    /// Return a vector of references to all acquisitions in the .mcd file (iterates over all slides and all panoramas).
    pub fn acquisitions(&self) -> Vec<&Acquisition<T>> {
        let mut acquisitions = HashMap::new();

        // This should be unnecessary - hopefully there is only one set of channels per dataset?
        for slide in self.slides.values() {
            for panorama in slide.panoramas() {
                for acquisition in panorama.acquisitions() {
                    acquisitions.insert(acquisition.id(), acquisition);
                }
            }
        }

        let mut ordered_acquisitions = Vec::new();
        for (_, acquisition) in acquisitions.drain() {
            ordered_acquisitions.push(acquisition);
        }

        ordered_acquisitions.sort_by_key(|a| a.id());

        ordered_acquisitions
    }

    /// Return an acquisition which matches the supplied `AcquisitionIdentifier` or None if no match found
    pub fn acquisition(&self, identifier: &AcquisitionIdentifier) -> Option<&Acquisition<T>> {
        for slide in self.slides.values() {
            for panorama in slide.panoramas() {
                for acquisition in panorama.acquisitions() {
                    match identifier {
                        AcquisitionIdentifier::Id(id) => {
                            if acquisition.id() == *id {
                                return Some(acquisition);
                            }
                        }
                        AcquisitionIdentifier::Order(order_number) => {
                            if acquisition.order_number() == *order_number {
                                return Some(acquisition);
                            }
                        }
                        AcquisitionIdentifier::Description(description) => {
                            if acquisition.description() == description {
                                return Some(acquisition);
                            }
                        }
                    }
                }
            }
        }

        None
    }

    /// Returns a vector of all channels present within any acquisition performed on the slide, sorted by channel order number.
    pub fn channels(&self) -> Vec<&AcquisitionChannel> {
        let mut channels = HashMap::new();

        // This should be unnecessary - hopefully there is only one set of channels per dataset?
        for slide in self.slides.values() {
            for panorama in slide.panoramas() {
                for acquisition in panorama.acquisitions() {
                    for channel in acquisition.channels() {
                        if !channels.contains_key(channel.name()) {
                            channels.insert(channel.name(), channel);
                        }
                    }
                }
            }
        }

        let mut ordered_channels = Vec::new();
        for (_, channel) in channels.drain() {
            ordered_channels.push(channel);
        }

        ordered_channels.sort_by_key(|a| a.order_number());

        ordered_channels
    }

    /// Returns an instance of `CalibrationFinal` with the specified ID, or None if none exists (this is always the case in version 1 of the Schema)
    pub fn calibration_final(&self, id: u16) -> Option<&CalibrationFinal> {
        self.calibration_finals.get(&id)
    }

    /// Returns an instance of `CalibrationParams` with the specified ID, or None if none exists (this is always the case in version 1 of the Schema)
    pub fn calibration_params(&self, id: u16) -> Option<&CalibrationParams> {
        self.calibration_params.get(&id)
    }

    /// Returns an instance of `CalibrationChannel` with the specified ID, or None if none exists (this is always the case in version 1 of the Schema)
    pub fn calibration_channels(&self, id: u16) -> Option<&CalibrationChannel> {
        self.calibration_channels.get(&id)
    }

    /// Returns an instance of `Calibration` with the specified ID, or None if none exists (this is always the case in version 1 of the Schema)
    pub fn calibration(&self, id: u16) -> Option<&Calibration> {
        self.calibrations.get(&id)
    }

    /// Returns an instance of `SlideFiducialMarks` with the specified ID, or None if none exists (this is always the case in version 1 of the Schema)
    pub fn slide_fiducal_marks(&self, id: u16) -> Option<&SlideFiducialMarks> {
        self.slide_fiducal_marks.get(&id)
    }

    /// Returns an instance of `SlideProfile` with the specified ID, or None if none exists (this is always the case in version 1 of the Schema)
    pub fn slide_profile(&self, id: u16) -> Option<&SlideProfile> {
        self.slide_profiles.get(&id)
    }

    /// Parse *.mcd format
    pub fn parse(reader: T, location: &str) -> Self {
        let mut mcd = MCD::new(reader, location);
        let combined_xml = mcd.xml();

        let mut file = std::fs::File::create("tmp.xml").unwrap();
        file.write_all(combined_xml.as_bytes()).unwrap();

        let mut parser = MCDParser::new(mcd);

        let mut reader = quick_xml::Reader::from_str(&combined_xml);
        let mut buf = Vec::with_capacity(BUF_SIZE);

        loop {
            match reader.read_event(&mut buf) {
                Ok(event) => {
                    parser.process(event);

                    // Check whether we are finished or have encounted a fatal error
                    match parser.current_state() {
                        ParserState::FatalError => {
                            let error = match parser.pop_error_back() {
                                // TODO: Probably a better way of doing this..
                                Some(value) => value,
                                None => String::from("unknown error"),
                            };

                            println!("An fatal error occurred when parsing: {}", error);
                            break;
                        }
                        ParserState::Finished => {
                            break;
                        }
                        _ => (),
                    }
                }
                Err(error) => {
                    println!("An error occurred when reading: {}", error);
                    break;
                }
            }

            buf.clear();
        }

        parser.mcd()
    }

    /// Parse *.mcd format where a temporary file is generated for faster access to channel images.
    ///
    /// Data is stored in the *.mcd file spectrum-wise which means to load a single image, the entire acquired acquisition must be loaded first.
    /// This method creates a temporary file (*.dcm) in the same location as the *.mcd file (if it doesn't already exist) which has the channel
    /// data stored image-wise. If this file is present and loaded, then `Mcd` will choose the fastest method to use to return the requested data.
    pub fn parse_with_dcm(reader: T, location: &str) -> Self {
        let mut mcd = Self::parse(reader, location);

        if std::fs::metadata(mcd.dcm_file()).is_err() {
            convert::convert(&mcd).unwrap();
        }

        convert::open(&mut mcd).unwrap();

        mcd
    }

    /// Returns the raw XML metadata stored in the .mcd file
    pub fn xml(&mut self) -> String {
        let chunk_size: i64 = 1000;
        let mut cur_offset: i64 = 0;

        let mut buf_u8 = vec![0; chunk_size.try_into().unwrap()];

        loop {
            let mut reader = self.reader.lock().unwrap();

            reader
                .seek(SeekFrom::End(-cur_offset - chunk_size))
                .unwrap();

            reader.read_exact(&mut buf_u8).unwrap();

            match std::str::from_utf8(&buf_u8) {
                Ok(_data) => {}
                Err(_error) => {
                    // Found the final chunk, so find the start point
                    let start_index = find_mcd_start(&buf_u8, chunk_size.try_into().unwrap());

                    let total_size = cur_offset + chunk_size - (start_index as i64);
                    buf_u8 = vec![0; total_size.try_into().unwrap()];

                    reader.seek(SeekFrom::End(-total_size)).unwrap();
                    reader.read_exact(&mut buf_u8).unwrap();

                    break;
                }
            }

            cur_offset += chunk_size;
        }

        let mut combined_xml = String::new();

        let mut buf_u16: Vec<u16> = vec![0; buf_u8.len() / 2];
        u16_from_u8(&mut buf_u16, &buf_u8);

        match String::from_utf16(&buf_u16) {
            Ok(data) => combined_xml.push_str(&data),
            Err(error) => {
                println!("{}", error)
            }
        }

        combined_xml
    }
}

#[rustfmt::skip]
impl<T: Seek + Read> Print for MCD<T> {
    fn print<W: fmt::Write + ?Sized>(&self, writer: &mut W, indent: usize) -> fmt::Result {
        writeln!(writer, "MCD File: {}", self.location)?;

        match self.xmlns.as_ref() {
            Some(xmlns) => writeln!(writer, "XML Namespace: {}", xmlns)?,
            None => {
                todo!()
            }
        }

        for slide in self.slides.values() {
            slide.print(writer, indent + 1)?;
        }

        let channels = self.channels();
        write!(writer, "{:indent$}", "", indent = indent)?;
        writeln!(writer, "{:-^1$}", "Channels", 25)?;

        for channel in channels {
            writeln!(
                writer,
                "{0: <2} | {1: <10} | {2: <10}",
                channel.order_number(),
                channel.name(),
                channel.label()
            )?;
        }

        Ok(())
    }
}

impl<T: Seek + Read> fmt::Display for MCD<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.print(f, 0)
    }
}

//pub struct MCDPublic {}

/*#[derive(Debug)]
pub enum ImageFormat {
    Png,
}*/

/// Represents a bounding rectangle
#[derive(Debug)]
pub struct BoundingBox<T> {
    /// Minimum x coordinate for the bounding rectangle
    pub min_x: T,
    /// Minimum y coordinate for the bounding rectangle
    pub min_y: T,
    /// Width of bounding rectangle
    pub width: T,
    /// Height of bounding rectangle
    pub height: T,
}

/// Represents a channel image (stored as a vector of f32).
/// If the run was stopped mid acquisition width*height != valid_pixels
pub struct ChannelImage {
    width: i32,
    height: i32,
    range: (f32, f32),
    valid_pixels: usize,
    data: Vec<f32>,
}

impl ChannelImage {
    /// Returns the width (in pixels) of the image
    pub fn width(&self) -> i32 {
        self.width
    }

    /// Returns the height (in pixels) of the image
    pub fn height(&self) -> i32 {
        self.height
    }

    /// Returns a pair (min, max) of f32 describing the limits of the detected intensities in the image
    pub fn intensity_range(&self) -> (f32, f32) {
        self.range
    }

    /// Returns whether the data is complete (true) or whether the data acquisition aborted (false)
    pub fn is_complete(&self) -> bool {
        self.valid_pixels == (self.width * self.height) as usize
    }

    /// Returns the number of valid pixels in the image. If the run was aborted part way through `num_valid_pixels() < width() * height()`
    pub fn num_valid_pixels(&self) -> usize {
        self.valid_pixels
    }

    /// Returns the detected intensity values for this channel
    pub fn intensities(&self) -> &[f32] {
        &self.data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs::File;

    use std::io::BufReader;

    #[test]
    fn test_all_in_folder() -> std::io::Result<()> {
        let paths = std::fs::read_dir("../test/").unwrap();

        for path in paths {
            let path = path?;

            if path.path().extension().unwrap() != "mcd" {
                println!("Skipping {:?} file.", path.path().extension().unwrap());
                continue;
            }

            let file = BufReader::new(File::open(path.path()).unwrap());
            let mut mcd = MCD::parse_with_dcm(file, path.path().to_str().unwrap());

            let _xml = mcd.xml();

            //println!("{}", xml);

            for acquisition in mcd.acquisitions() {
                println!("[{}] {}", acquisition.id(), acquisition.description());
            }

            let acquisition = mcd.acquisitions()[0];

            let acquisition = mcd
                .acquisition(&AcquisitionIdentifier::Description(
                    acquisition.description().to_string(),
                ))
                .unwrap();

            let x_channel = acquisition
                .channel_data(&ChannelIdentifier::Name("X".to_string()))
                .unwrap();

            println!("Loaded X Channel : {:?}", x_channel.num_valid_pixels());

            for channel in acquisition.channels() {
                println!(
                    "[Channel {}] {} | {}",
                    channel.id(),
                    channel.label(),
                    channel.name()
                );
            }

            let channel_identifier = ChannelIdentifier::Name("Ir(191)".to_string());
            let overview_image = mcd.slides()[0]
                .create_overview_image(15000, Some((&channel_identifier, Some(100.0))))
                .unwrap();

            overview_image.save("overview.png").unwrap();
        }

        Ok(())
    }
}
