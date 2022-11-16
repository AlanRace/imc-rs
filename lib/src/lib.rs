#![warn(missing_docs)]
#![warn(clippy::unwrap_used)]

//! This library provides a means of accessing imaging mass cytometry (Fluidigm) data stored in the (*.mcd) format.
//!
//! # Example
//!
//! To run this example, make sure to first download the test file [20200612_FLU_1923.mcd](https://zenodo.org/record/4110560/files/data/20200612_FLU_1923/20200612_FLU_1923.mcd?download=1) to the `test/` folder.
//!
//! ```
//! extern crate imc_rs;
//!
//! use imc_rs::MCD;
//! use std::io::BufReader;
//! use std::fs::File;
//!
//! fn main() {
//!     let filename = "../test/20200612_FLU_1923.mcd";
//!
//!     let mcd = MCD::from_path(filename).unwrap();
//!
//!     // Optionally we can load/create the .dcm file for fast access to images
//!     let mcd = mcd.with_dcm().unwrap();
//!
//! }
//! ```

pub mod convert;
/// Errors associated with parsing IMC data
pub mod error;
pub(crate) mod mcd;
/// Transformations (e.g. affine) used for converting
pub mod transform;

mod acquisition;
mod calibration;
mod channel;
mod panorama;
mod slide;

/// Provides methods for reading in cell segmentation data from Halo
pub mod halo;

pub use self::acquisition::{Acquisition, AcquisitionIdentifier, Acquisitions};
pub use self::channel::{AcquisitionChannel, ChannelIdentifier};
pub use self::panorama::Panorama;
pub use self::slide::Slide;

use error::{MCDError, Result};
use image::io::Reader as ImageReader;
use std::convert::TryInto;
use std::fmt;
use std::fs::File;
use std::io::{BufReader, BufWriter, Cursor, Read, Seek, SeekFrom};

use std::ops::DerefMut;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use std::collections::HashMap;

use calibration::{Calibration, CalibrationChannel, CalibrationFinal, CalibrationParams};
use mcd::{MCDParser, ParserState};

use image::{DynamicImage, ImageFormat, RgbaImage};
use slide::{SlideFiducialMarks, SlideProfile};
use transform::AffineTransform;

const BUF_SIZE: usize = 4096;

/// Print to `writer` trait
pub trait Print {
    /// Formats and prints to `writer`
    fn print<W: fmt::Write + ?Sized>(&self, writer: &mut W, indent: usize) -> fmt::Result;
}

/// Represents property of having an optical image
pub struct OpticalImage<R> {
    reader: Arc<Mutex<BufReader<R>>>,

    // TODO: Why are we using i64 here?
    start_offset: i64,
    end_offset: i64,
    image_format: ImageFormat,
}

impl<R: Read + Seek> OpticalImage<R> {
    /// Returns whether an optical image is present
    //fn has_image(&self) -> bool;
    /// Returns the binary data for the image, exactly as stored in the .mcd file
    pub fn image_data(&self) -> Result<Vec<u8>> {
        let mut reader = self.reader.lock().or(Err(MCDError::PoisonMutex))?;

        let start_offset = self.start_offset();
        let image_size = self
            .image_size()
            .try_into()
            .or(Err(MCDError::InvalidOffset {
                offset: start_offset,
            }))?;

        let mut buf_u8 = vec![0; image_size];

        match reader.seek(SeekFrom::Start(start_offset as u64)) {
            Ok(_seek) => match reader.read_exact(&mut buf_u8) {
                Ok(()) => Ok(buf_u8),
                Err(error) => Err(error.into()),
            },
            Err(error) => Err(error.into()),
        }
    }

    /// Returns the dimensions of the images in pixels as a tuple (width, height)
    pub fn dimensions(&self) -> Result<(u32, u32)> {
        let mut guard = self.reader.lock().or(Err(MCDError::PoisonMutex))?;
        let reader: &mut BufReader<R> = guard.deref_mut();

        let start_offset = self.start_offset();
        reader.seek(SeekFrom::Start(start_offset as u64))?;

        let mut reader = ImageReader::new(reader);
        reader.set_format(self.image_format());

        match reader.into_dimensions() {
            Ok(dims) => Ok(dims),
            Err(error) => Err(MCDError::from(error)),
        }
    }

    /// Returns a decoded RgbaImage of the panorama image
    pub fn as_rgba8(&self) -> Result<RgbaImage> {
        // match self.dynamic_image()? {
        //     DynamicImage::ImageRgba8(rgba8) => Ok(rgba8),
        //     DynamicImage::ImageRgb8(rgb8) => Ok(DynamicImage::ImageRgb8(rgb8).into_rgba8()),
        //     _ => panic!("Unexpected DynamicImage type"),
        // }
        Ok(self.dynamic_image()?.into_rgba8())
    }

    fn dynamic_image(&self) -> Result<DynamicImage> {
        let mut reader = ImageReader::new(Cursor::new(self.image_data()?));
        reader.set_format(self.image_format);

        // TODO: Deal with this error properly
        Ok(reader.decode()?)
    }
}

impl<R> OpticalImage<R> {
    fn start_offset(&self) -> i64 {
        self.start_offset + 161
    }

    fn image_size(&self) -> i64 {
        self.end_offset - self.start_offset()
    }

    /// Returns the format of the stored optical image
    pub fn image_format(&self) -> ImageFormat {
        self.image_format
    }
}

/// Represents an image which is acquired on a slide
pub trait OnSlide {
    /// Returns the bounding box encompasing the image area on the slide (in μm)
    fn slide_bounding_box(&self) -> BoundingBox<f64>;
    /// Returns the affine transformation from pixel coordinates within the image to to the slide coordinates (μm)
    fn to_slide_transform(&self) -> AffineTransform<f64>;
}

/// Represents a region of an image (in pixels)
#[derive(Debug, Clone, Copy)]
pub struct Region {
    /// x-position of the top left corner of the region
    pub x: u32,
    /// y-position of the top left corner of the region
    pub y: u32,
    /// width of the region in pixels
    pub width: u32,
    /// height of the region in pixels
    pub height: u32,
}

/// Represents a imaging mass cytometry (*.mcd) file.
#[derive(Debug)]
pub struct MCD<R> {
    reader: Arc<Mutex<std::io::BufReader<R>>>,
    location: Option<PathBuf>,

    xmlns: Option<String>,

    slides: HashMap<u16, Slide<R>>,
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

impl MCD<File> {
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<MCD<File>> {
        let mut mcd = MCD::parse(File::open(&path)?)?;
        mcd.set_location(path);

        Ok(mcd)
    }

    /// Returns the location (path) of the .mcd file
    pub fn location(&self) -> Option<&Path> {
        Some(self.location.as_ref()?.as_path())
    }

    /// Returns the location (path) of the .mcd file
    pub fn set_location<P: AsRef<Path>>(&mut self, location: P) {
        let mut path_buf = PathBuf::new();
        path_buf.push(location);

        self.location = Some(path_buf);
    }

    pub(crate) fn dcm_file(&self) -> Option<PathBuf> {
        let mut path = PathBuf::from(self.location()?);
        path.set_extension("dcm");

        Some(path)
    }

    /// Use a temporary file for faster access to channel images.
    ///
    /// If this file does not already exist, then it is created.
    ///
    /// Data is stored in the *.mcd file spectrum-wise which means to load a single image, the entire acquired acquisition must be loaded first.
    /// This method creates a temporary file (*.dcm) in the same location as the *.mcd file (if it doesn't already exist) which has the channel
    /// data stored image-wise. If this file is present and loaded, then `Mcd` will choose the fastest method to use to return the requested data.
    ///
    /// # Errors
    ///
    /// If the location is not set either automatically via [`MCD::from_path`] or manually via [`MCD::set_location`] then a [`MCDError::LocationNotSpecified`]
    /// will occur.
    pub fn with_dcm(mut self) -> Result<Self> {
        if std::fs::metadata(self.dcm_file().ok_or(MCDError::LocationNotSpecified)?).is_err() {
            let dcm_file =
                std::fs::File::create(self.dcm_file().ok_or(MCDError::LocationNotSpecified)?)?;
            let mut dcm_file = BufWriter::new(dcm_file);

            convert::convert(&self, &mut dcm_file)?;
        }

        convert::open(&mut self)?;

        Ok(self)
    }
}

impl<R: Read + Seek> MCD<R> {
    fn new(reader: R) -> Self {
        MCD {
            reader: Arc::new(Mutex::new(BufReader::new(reader))),
            location: None,
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

    /// Parse *.mcd format
    pub fn parse(reader: R) -> Result<Self> {
        let mcd = MCD::new(reader);
        let combined_xml = mcd.xml()?;

        // let mut file = std::fs::File::create("tmp.xml").unwrap();
        // file.write_all(combined_xml.as_bytes())?;

        let mut parser = MCDParser::new(mcd);

        // println!("Found combined XML {}", combined_xml);

        let mut reader = quick_xml::Reader::from_str(&combined_xml);
        // let mut buf = Vec::with_capacity(BUF_SIZE);

        loop {
            match reader.read_event() {
                Ok(event) => {
                    // println!("Event: {:?}", event);
                    parser.process(event);

                    // Check whether we are finished or have encounted a fatal error
                    match parser.current_state() {
                        ParserState::FatalError => {
                            match parser.pop_error_back() {
                                // TODO: Probably a better way of doing this..
                                Some(value) => return Err(value),
                                None => println!(
                                    "A fatal error occurred when parsing, but it wasn't recorded"
                                ),
                            }

                            break;
                        }
                        ParserState::Finished => {
                            break;
                        }
                        _ => (),
                    }
                }
                Err(error) => {
                    return Err(error.into());
                    // println!("An error occurred when reading: {}", error);
                    // break;
                }
            }

            // buf.clear();
        }

        let mcd = parser.mcd();

        if mcd.slides().is_empty() {
            Err(MCDError::NoSlidePresent)
        } else {
            Ok(mcd)
        }
    }

    /// Returns the raw XML metadata stored in the .mcd file
    pub fn xml(&self) -> Result<String> {
        let chunk_size_i64: i64 = 1000;
        let mut cur_offset: i64 = 0;

        let chunk_size = chunk_size_i64.try_into()?;

        let mut buf_u8 = vec![0; chunk_size];

        loop {
            let mut reader = self.reader.lock().or(Err(MCDError::PoisonMutex))?;

            reader.seek(SeekFrom::End(-cur_offset - chunk_size_i64))?;

            reader.read_exact(&mut buf_u8)?;

            match std::str::from_utf8(&buf_u8) {
                Ok(_data) => {}
                Err(_error) => {
                    // Found the final chunk, so find the start point
                    let start_index = find_mcd_start(&buf_u8, chunk_size);

                    let total_size = cur_offset + chunk_size_i64 - (start_index as i64);
                    buf_u8 = vec![0; total_size.try_into()?];

                    reader.seek(SeekFrom::End(-total_size))?;
                    reader.read_exact(&mut buf_u8)?;

                    break;
                }
            }

            cur_offset += chunk_size_i64;
        }

        let mut combined_xml = String::new();

        let mut buf_u16: Vec<u16> = vec![0; buf_u8.len() / 2];
        u16_from_u8(&mut buf_u16, &buf_u8);

        let data = String::from_utf16(&buf_u16)?;
        combined_xml.push_str(&data);

        Ok(combined_xml)
    }
}

impl<R> MCD<R> {
    pub(crate) fn reader(&self) -> &Arc<Mutex<BufReader<R>>> {
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
    pub fn slide(&self, id: u16) -> Option<&Slide<R>> {
        self.slides.get(&id)
    }

    /// Returns a vector of references to slides sorted by ID number. This allocates a new vector on each call.
    pub fn slides(&self) -> Vec<&Slide<R>> {
        let mut slides = Vec::new();

        for id in self.slide_ids() {
            slides.push(
                self.slide(id)
                    .expect("Should only be finding slides that exist"),
            );
        }

        slides
    }

    fn slides_mut(&mut self) -> &mut HashMap<u16, Slide<R>> {
        &mut self.slides
    }

    /// Return a vector of references to all acquisitions in the .mcd file (iterates over all slides and all panoramas).
    pub fn acquisitions(&self) -> Vec<&Acquisition<R>> {
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
    pub fn acquisition<A: Into<AcquisitionIdentifier>>(
        &self,
        identifier: A,
    ) -> Option<&Acquisition<R>> {
        let identifier = identifier.into();

        for slide in self.slides.values() {
            for panorama in slide.panoramas() {
                for acquisition in panorama.acquisitions() {
                    match &identifier {
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

    /// Returns a list of acquisitions which are at least partially contained within the specified bounding box.
    pub fn acquisitions_in(&self, region: &BoundingBox<f64>) -> Vec<&Acquisition<R>> {
        let mut acquisitions = Vec::new();

        for slide in self.slides.values() {
            for panorama in slide.panoramas() {
                for acquisition in panorama.acquisitions() {
                    if acquisition.in_region(region) {
                        acquisitions.push(acquisition);
                    }
                }
            }
        }

        acquisitions
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

        ordered_channels.sort_by_key(|a| a.label());

        ordered_channels
    }

    /// Returns a vector of all channels, excluding those from the acquisitions with names matching those specified
    pub fn channels_excluding(&self, exclusion_list: Vec<&str>) -> Vec<&AcquisitionChannel> {
        let mut channels = HashMap::new();

        // This should be unnecessary - hopefully there is only one set of channels per dataset?
        for slide in self.slides.values() {
            for panorama in slide.panoramas() {
                for acquisition in panorama.acquisitions() {
                    if !exclusion_list.contains(&acquisition.description()) {
                        for channel in acquisition.channels() {
                            if !channels.contains_key(channel.name())
                                && channel.name() != "X"
                                && channel.name() != "Y"
                                && channel.name() != "Z"
                            {
                                channels.insert(channel.name(), channel);
                            }
                        }
                    }
                }
            }
        }

        let mut ordered_channels = Vec::new();
        for (_, channel) in channels.drain() {
            ordered_channels.push(channel);
        }

        ordered_channels.sort_by_key(|a| a.label().to_ascii_lowercase());

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
}

#[rustfmt::skip]
impl<R> Print for MCD<R> {
    fn print<W: fmt::Write + ?Sized>(&self, writer: &mut W, indent: usize) -> fmt::Result {
        // writeln!(writer, "MCD File: {}", self.location)?;

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

impl<R> fmt::Display for MCD<R> {
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
#[derive(Debug, Clone)]
pub struct BoundingBox<T: num_traits::Num + Copy> {
    /// Minimum x coordinate for the bounding rectangle
    pub min_x: T,
    /// Minimum y coordinate for the bounding rectangle
    pub min_y: T,
    /// Width of bounding rectangle
    pub width: T,
    /// Height of bounding rectangle
    pub height: T,
}

impl<T: num_traits::Num + Copy> BoundingBox<T> {
    /// Maximum x coordinate for the bounding rectangle
    pub fn max_x(&self) -> T {
        self.min_x + self.width
    }

    /// Maximum y coordinate for the bounding rectangle
    pub fn max_y(&self) -> T {
        self.min_y + self.height
    }
}

/// Represents a channel image (stored as a vector of f32).
/// If the run was stopped mid acquisition width*height != valid_pixels
pub struct ChannelImage {
    region: Region,

    range: (f32, f32),
    valid_pixels: usize,
    data: Vec<f32>,
}

impl ChannelImage {
    /// Returns the width (in pixels) of the image
    pub fn width(&self) -> u32 {
        self.region.width
    }

    /// Returns the height (in pixels) of the image
    pub fn height(&self) -> u32 {
        self.region.height
    }

    /// Returns a pair (min, max) of f32 describing the limits of the detected intensities in the image
    pub fn intensity_range(&self) -> (f32, f32) {
        self.range
    }

    /// Returns whether the data is complete (true) or whether the data acquisition aborted (false)
    pub fn is_complete(&self) -> bool {
        self.valid_pixels == (self.region.width * self.region.height) as usize
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

struct Cell {
    markers: Vec<Summary<f32>>,
}

#[derive(Debug, Clone)]
struct Summary<T> {
    mean: T,
    std: T,
    range: (T, T),
    median: T,
}

struct Phenotype {
    description: String,
    rule: Rule,
}

impl Phenotype {
    pub fn matches(
        &self,
        channels: &[AcquisitionChannel],
        spectrum: &[Summary<f32>],
    ) -> Result<bool> {
        self.rule.matches(channels, spectrum)
    }
}

impl AsRef<Rule> for Phenotype {
    fn as_ref(&self) -> &Rule {
        &self.rule
    }
}

#[derive(Debug, Clone)]
enum Direction {
    Above,
    Below,
}

#[derive(Debug, Clone)]
enum Interval {
    Closed,
    Open,
}

#[derive(Debug, Clone)]
enum Rule {
    Threshold(ChannelIdentifier, f32, Direction, Interval),
    And(Box<Rule>, Box<Rule>),
    Or(Box<Rule>, Box<Rule>),
}

impl Rule {
    pub fn and<A: AsRef<Rule>, B: AsRef<Rule>>(left: A, right: B) -> Self {
        Self::And(
            Box::new(left.as_ref().clone()),
            Box::new(right.as_ref().clone()),
        )
    }

    pub fn matches(
        &self,
        channels: &[AcquisitionChannel],
        spectrum: &[Summary<f32>],
    ) -> Result<bool> {
        match self {
            Rule::Threshold(identifier, threshold, direction, interval) => {
                for (channel, summary) in channels.iter().zip(spectrum) {
                    if channel.is(identifier) {
                        match (direction, interval) {
                            (Direction::Above, Interval::Closed) => {
                                return Ok(summary.mean >= *threshold)
                            }
                            (Direction::Above, Interval::Open) => {
                                return Ok(summary.mean > *threshold)
                            }
                            (Direction::Below, Interval::Closed) => {
                                return Ok(summary.mean <= *threshold)
                            }
                            (Direction::Below, Interval::Open) => {
                                return Ok(summary.mean < *threshold)
                            }
                        }
                    }
                }

                // We didn't find the channel in the list of channels, so something went wrong
                Err(MCDError::InvalidChannel {
                    channel: identifier.clone(),
                })
            }
            Rule::And(left, right) => {
                Ok(left.matches(channels, spectrum)? && right.matches(channels, spectrum)?)
            }
            Rule::Or(left, right) => {
                Ok(left.matches(channels, spectrum)? || right.matches(channels, spectrum)?)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use image::{GenericImageView, ImageBuffer, Pixel, Rgba};
    use tiff::decoder::Decoder;

    use super::*;

    use std::time::Instant;

    #[test]
    fn test_load() -> Result<()> {
        let filename = "../test/20200612_FLU_1923.mcd";

        let start = Instant::now();
        let mcd = MCD::from_path(filename)?;
        println!("Time taken to parse .mcd: {:?}", start.elapsed());

        // Optionally we can load/create the .dcm file for fast access to images
        let start = Instant::now();
        let mcd = mcd.with_dcm()?;
        println!("Time taken to parse .dcm: {:?}", start.elapsed());

        let start = Instant::now();
        let roi_001 = mcd.acquisition("ROI_001").unwrap();
        println!("Time taken to find acquisition: {:?}", start.elapsed());

        let dna = roi_001.channel(ChannelIdentifier::label("DNA1")).unwrap();

        // Available here: https://zenodo.org/record/4139443#.Y2okw0rMLmE

        let img_file = File::open("../test/20200612_FLU_1923-01_full_mask.tiff")?;
        let mut decoder = Decoder::new(img_file).expect("Cannot create decoder");

        let (width, height) = decoder.dimensions().unwrap();

        let mut cells: HashMap<u16, Vec<_>> = HashMap::new();
        let image = decoder.read_image().unwrap();

        match image {
            tiff::decoder::DecodingResult::U16(cell_data) => {
                for y in 0..height {
                    for x in 0..width {
                        let index = (y * width) + x;

                        if cell_data[index as usize] > 0 {
                            match cells.entry(cell_data[index as usize]) {
                                std::collections::hash_map::Entry::Occupied(mut entry) => {
                                    entry.get_mut().push((x, y));
                                }
                                std::collections::hash_map::Entry::Vacant(entry) => {
                                    entry.insert(vec![(x, y)]);
                                }
                            }
                        }
                    }
                }
            }
            _ => todo!(),
        }

        println!("Detected {} cells.", cells.len());
        println!("Time taken to detect cells: {:?}", start.elapsed());

        let cell = cells.get(&1).unwrap();

        println!(
            "{:?}",
            roi_001
                .channels()
                .iter()
                .map(|channel| channel.label())
                .collect::<Vec<_>>()
        );

        // cell types: https://github.com/camlab-bioml/astir/blob/master/tests/test-data/jackson-2020-markers.yml

        let phenotype_histone = Phenotype {
            description: "Histone+".to_string(),
            rule: Rule::Threshold(
                ChannelIdentifier::label("HistoneH3"),
                2.0,
                Direction::Above,
                Interval::Open,
            ),
        };

        let phenotype_cd16 = Phenotype {
            description: "CD16+".to_string(),
            rule: Rule::Threshold(
                ChannelIdentifier::label("CD16"),
                1.0,
                Direction::Above,
                Interval::Open,
            ),
        };

        let combined = Phenotype {
            description: "combined".to_string(),
            rule: Rule::and(&phenotype_histone, &phenotype_cd16),
        };

        for (index, cell) in cells {
            let mut spectrum = vec![Vec::with_capacity(cell.len()); roi_001.channels().len()];

            for (x, y) in cell {
                spectrum
                    .iter_mut()
                    .zip(roi_001.spectrum(x as usize, y as usize)?.iter())
                    .for_each(|(s, i)| s.push(*i));
            }

            let summaries = spectrum
                .drain(..)
                .map(|mut intensities| {
                    intensities.sort_by(|a, b| a.partial_cmp(b).unwrap());

                    // println!("{:?}", intensities);

                    let mean: f32 = intensities.iter().sum::<f32>() / intensities.len() as f32;

                    let variance: f32 =
                        intensities.iter().map(|x| (*x - mean).powi(2)).sum::<f32>()
                            / intensities.len() as f32;

                    let median = if intensities.len() % 2 == 0 {
                        let mid_point = intensities.len() / 2;

                        (intensities[mid_point] + intensities[mid_point - 1]) * 0.5
                    } else {
                        intensities[(intensities.len() - 1) / 2]
                    };

                    Summary {
                        mean,
                        median,
                        range: (intensities[0], intensities[intensities.len() - 1]),
                        std: variance.sqrt(),
                    }
                })
                .collect::<Vec<_>>();

            // println!("{:?}", summaries);
            // if combined.matches(roi_001.channels(), &spectrum) {
            // println!(
            //     "[{}] {:?} {:?} {:?}",
            //     index,
            //     phenotype_histone.matches(roi_001.channels(), &summaries),
            //     phenotype_cd16.matches(roi_001.channels(), &summaries),
            //     combined.matches(roi_001.channels(), &summaries)
            // );
            // }
        }

        // println!("{:?}", cell);

        Ok(())
    }

    #[test]
    fn test_all_in_folder() -> Result<()> {
        let paths = std::fs::read_dir("test/").unwrap();

        for path in paths {
            let path = path?;

            if path.path().extension().unwrap() != "mcd" {
                println!("Skipping {:?} file.", path.path().extension().unwrap());
                continue;
            }

            let mcd = MCD::from_path(path.path())?;
        }

        Ok(())
    }

    // #[test]
    // fn test_all_in_folder() -> Result<()> {
    //     let paths = std::fs::read_dir("test/").unwrap();

    //     for path in paths {
    //         let path = path?;

    //         if path.path().extension().unwrap() != "mcd" {
    //             println!("Skipping {:?} file.", path.path().extension().unwrap());
    //             continue;
    //         }

    //         let mcd = MCD::from_path(path.path())?.with_dcm()?;

    //         // let overview_image = mcd.slides()[0].create_overview_image(7500, None).unwrap();

    //         // overview_image.save("overview.png").unwrap();

    //         //let _xml = mcd.xml()?;

    //         //println!("{}", xml);

    //         for acquisition in mcd.acquisitions() {
    //             println!("[{}] {}", acquisition.id(), acquisition.description());
    //         }

    //         let acquisition = mcd.acquisitions()[0];

    //         let acquisition = mcd
    //             .acquisition(&AcquisitionIdentifier::Description(
    //                 acquisition.description().to_string(),
    //             ))
    //             .unwrap();

    //         let x_channel = acquisition
    //             .channel_image(&ChannelIdentifier::Name("X".to_string()), None)
    //             .unwrap();

    //         println!("Loaded X Channel : {:?}", x_channel.num_valid_pixels());

    //         for channel in mcd.channels_excluding(vec!["ROI 12", "ROI 13", "ROI 14", "ROI 16"]) {
    //             println!(
    //                 "[Channel {}] {} | {}",
    //                 channel.id(),
    //                 channel.label(),
    //                 channel.name()
    //             );
    //         }

    //         let channel_identifier = ChannelIdentifier::Name("Ir(191)".to_string());
    //         println!("Subimage");
    //         let data = acquisition.channel_images(
    //             &[channel_identifier.clone()],
    //             // None,
    //             Some(Region {
    //                 x: 1000,
    //                 y: 1000,
    //                 width: 500,
    //                 height: 500,
    //             }),
    //         )?;

    //         let data = &data[0];

    //         let mut acq_image: ImageBuffer<Rgba<u8>, Vec<u8>> =
    //             ImageBuffer::new(data.width(), data.height());

    //         let mut index = 0;
    //         let max_value = 20.0;
    //         for y in 0..data.height() {
    //             if index >= data.valid_pixels {
    //                 break;
    //             }

    //             for x in 0..data.width() {
    //                 if index >= data.valid_pixels {
    //                     break;
    //                 }

    //                 let g = ((data.data[index] / max_value) * 255.0) as u8;
    //                 let g = g as f64 / 255.0;

    //                 let cur_pixel = acq_image.get_pixel_mut(x as u32, y as u32).channels_mut();
    //                 cur_pixel[1] = (g * 255.0) as u8;
    //                 cur_pixel[3] = 255;

    //                 index += 1;
    //             }
    //         }

    //         // acq_image.save("dna.png").unwrap();

    //         // let start = Instant::now();
    //         // let mut image_map = HashMap::new();

    //         // for acquisition in mcd.acquisitions() {
    //         //     if let Ok(data) = acquisition.channel_image(
    //         //         &channel_identifier,
    //         //         // None
    //         //         Some(Region {
    //         //             x: 1000,
    //         //             y: 1000,
    //         //             width: 500,
    //         //             height: 500,
    //         //         }),
    //         //     ) {
    //         //         image_map.insert(format!("{}", acquisition.id()), ChannelImage(data));
    //         //     }
    //         // }

    //         // let duration = start.elapsed();

    //         // println!("Time elapsed loading data is: {:?}", duration);
    //     }

    //     Ok(())
    // }
}
