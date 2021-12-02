#![feature(hash_set_entry)]
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

use std::fmt;
use std::io::{Read, Seek};

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use std::collections::HashMap;

use acquisition::AcquisitionIdentifier;
use mcd::{MCDParser, ParserState};

use image::{ImageFormat, RgbaImage};
use transform::AffineTransform;

const BUF_SIZE: usize = 4096;

/// Print to `writer` trait
pub trait Print {
    /// Formats and prints to `writer`
    fn print<W: fmt::Write + ?Sized>(&self, writer: &mut W, indent: usize) -> fmt::Result;
}

/// Represents property of having an optical image
pub trait HasOpticalImage {
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

    /// Parse *.mcd format
    pub fn parse(reader: T, location: &str) -> Self {
        let mcd = MCD::new(reader, location);
        let mut parser = MCDParser::new(mcd);

        let combined_xml = parser.get_xml();

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
    use image::{ImageBuffer, Pixel, Rgba};

    use super::*;

    use core::panic;
    use std::fs::File;
    use std::time::Instant;

    use std::io::BufReader;

    #[test]
    fn it_works() -> std::io::Result<()> {
        let start = Instant::now();
        //let filename = "/home/alan/Documents/Work/IMC/set1.mcd";
        let filename =
        //    "/media/alan/Seagate Portable Drive/AZ/Gemcitabine/20181002_IMC_Gemcitabine_testpanel.mcd";
            "/home/alan/Documents/Nicole/Salmonella/2019-10-25_Salmonella_final_VS+WT.mcd";

        let duration = start.elapsed();

        //while parser.has_errors() {
        //    println!("{}", parser.pop_error_front().unwrap());
        //}

        let file = BufReader::new(File::open(filename).unwrap());
        let mcd = MCD::parse_with_dcm(file, filename);

        //println!("{:?}", mcd.slide);
        //println!("{:?}", mcd.panoramas);
        //for (id, acquisition) in &mcd.acquisitions {
        //    println!("{:?}: {:?}", id, acquisition);
        //}
        //println!("{:?}", mcd.acquisitions[0]);
        //println!("{:?}", mcd.roi_points[0]);
        //std::fs::write("tmp.xml", combined_xml).expect("Unable to write file");

        //std::fs::write("tmp.png", mcd.panoramas.get("8").unwrap().get_image(&mut file).unwrap())
        //    .expect("Unable to write file");

        println!("Time elapsed when parsing is: {:?}", duration);

        /*let acquisition = mcd
            .get_slide(&1)
            .unwrap()
            .get_panorama(&3)
            .unwrap()
            .get_acquisition(&1)
            .unwrap();
        println!("{}", acquisition);
        let point = acquisition.to_slide_transform().transform_point(1.0, 1.0);
        println!("Transformed point = {:?}", point);


        println!("Bounding box = {:?}", acquisition.slide_bounding_box());

        std::fs::write("tmp.png", acquisition.get_after_ablation_image().unwrap())
            .expect("Unable to write file");*/

        let _slide = mcd.slide(1).unwrap();

        //convert::convert(&mcd)?;

        //println!("{:?}", mcd);

        //return Ok(());

        //std::fs::write("slide.jpeg", slide.get_image().unwrap())
        //    .expect("Unable to write file");

        /*

        let offset_x = (bounding_box.min_x / 10.0).round() as u32;
        let offset_y = ((25000.0 - bounding_box.min_y - bounding_box.height) / 10.0).round() as u32;
        let width = (bounding_box.width / 10.0).round() as u32;
        let height = (bounding_box.height / 10.0).round() as u32;

        let panorama_image = panorama_image.resize_exact(width, height, FilterType::Nearest).to_rgb8();

        if offset_x + width > 7500 || offset_y + height > 2500 {
            continue;
        }

        for y in 0..height {
            for x in 0..width {
                resized_image.put_pixel(offset_x + x, offset_y + y, *panorama_image.get_pixel(x, y));
            }
        }*/

        //panorama_image.resize_exact()

        //let transform = panorama.to_slide_transform();

        let output_location = "/home/alan/Documents/Nicole/Salmonella/";
        let _path = std::path::Path::new(output_location);

        for acquisition in mcd.acquisitions() {
            println!("[{}] {}", acquisition.id(), acquisition.description());
        }

        let acquisition = mcd
            .acquisition(&AcquisitionIdentifier::Description("ROI 10".to_string()))
            .unwrap();

        let x_channel = acquisition
            .channel_data(&ChannelIdentifier::Name("X".to_string()))
            .unwrap();

        println!("Loaded X Channel : {:?}", x_channel.num_valid_pixels());

        for channel in acquisition.channels() {
            println!("[{}] {}", channel.label(), channel.name());
        }

        let data = acquisition
            .channel_data(&ChannelIdentifier::Label("Ki67_B56".to_string()))
            .unwrap();

        let mut acq_image: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::new(data.width as u32, data.height as u32);

        let mut index = 0;
        let max_value = 20.0;
        for y in 0..data.height {
            if index >= data.valid_pixels {
                break;
            }

            for x in 0..data.width {
                if index >= data.valid_pixels {
                    break;
                }

                let g = ((data.data[index] / max_value) * 255.0) as u8;
                let g = g as f64 / 255.0;

                let cur_pixel = acq_image.get_pixel_mut(x as u32, y as u32).channels_mut();
                cur_pixel[1] = (g * 255.0) as u8;
                cur_pixel[3] = 255;

                index += 1;
            }
        }

        acq_image.save("ki67.png").unwrap();

        //let cell_data = halo::parse_from_path("/home/alan/Documents/Nicole/Object data for phenotype correlation/VS_object data_final.csv")?;

        let positive_header = cell_data.header("Ki67_B56(Dy162Di) Positive").unwrap();
        let positive_cell = cell_data
            .column_data(positive_header.column_number())
            .unwrap();

        let positive_cell = match positive_cell {
            halo::ColumnData::Binary(data) => data,
            _ => {
                panic!("Wrong type of data in cell?");
            }
        };

        for (index, boundary) in cell_data.boundaries().enumerate() {
            //println!("[{}] {:?}", index, boundary);

            if positive_cell[index] {
                for y in [0, boundary.height] {
                    for x in 0..boundary.width {
                        let x = x + boundary.min_x;
                        let y = y + boundary.min_y;

                        if x < 0 || y < 0 || x >= data.width.into() || y >= data.height.into() {
                            continue;
                        }

                        let cur_pixel = acq_image.get_pixel_mut(x as u32, y as u32).channels_mut();
                        cur_pixel[0] = 255.0 as u8;
                    }
                }

                for x in [0, boundary.width] {
                    for y in 0..boundary.height {
                        let x = x + boundary.min_x;
                        let y = y + boundary.min_y;

                        if x < 0 || y < 0 || x >= data.width.into() || y >= data.height.into() {
                            continue;
                        }

                        let cur_pixel = acq_image.get_pixel_mut(x as u32, y as u32).channels_mut();
                        cur_pixel[0] = 255.0 as u8;
                    }
                }
            }

            /*if index > 10 {
                break;
            }*/
        }

        acq_image.save("ki67_with_markers.png").unwrap();

        /*for panorama in slide.panoramas() {
            let panorama_image = panorama.image();
            panorama_image
                .save(path.join(format!("{}.png", panorama.description())))
                .unwrap();

            for acquisition in panorama.acquisitions() {
                let acquisition_image = acquisition.before_ablation_image();
                acquisition_image
                    .save(format!(
                        "{}_{}.png",
                        panorama.description(),
                        acquisition.description()
                    ))
                    .unwrap();

                let cur_path = path.join(format!(
                    "{}_{}.txt",
                    panorama.description(),
                    acquisition.description()
                ));
                let mut f = File::create(cur_path).expect("Unable to create file");

                let transform = acquisition.to_slide_transform();
                let transform = transform.to_slide_matrix();
                writeln!(f, "{},{}", acquisition.width(), acquisition.height())?;
                writeln!(
                    f,
                    "{},{},{},{},{},{}",
                    transform.m11,
                    transform.m12,
                    transform.m13,
                    transform.m21,
                    transform.m22,
                    transform.m23
                )?;
            }
        }*/

        //let resized_image = slide.create_overview_image(15000);
        //resized_image
        //    .save(path.join("slide_overview.jpeg"))
        //    .unwrap();

        /*
        let mut points = Vec::new();
        points.push(Vector2::new(20.0, 10.0));
        points.push(Vector2::new(20.0, 20.0));
        points.push(Vector2::new(40.0, 10.0));
        //points.push(Vector2::new(40.0, 20.0));
        //points.push(Vector3::new(20.0, 10.0, 0.0));

        let mut points2 = Vec::new();
        points2.push(Vector2::new(40.0, 5.0));
        points2.push(Vector2::new(40.0, 10.0));
        points2.push(Vector2::new(80.0, 5.0));
        //points2.push(Vector2::new(80.0, 10.0));
        //points2.push(Vector3::new(20.0, 10.0, 0.0));

        let transform = AffineTransform::from_points(points, points2);*/

        //println!("{}", combined_xml);

        Ok(())
    }
}
