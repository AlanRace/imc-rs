#![warn(missing_docs)]
#![warn(rustdoc::missing_doc_code_examples)]

//! This library provides a means of accessing imaging mass cytometry (Fluidigm) data stored in the (*.mcd) format.
//!
//! # Example
//!
//! ```rust
//! extern crate imc_rs;
//!
//! use imc_rs::;
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

pub use self::acquisition::Acquisition;
pub use self::panorama::Panorama;
pub use self::slide::Slide;

use std::fmt;

use byteorder::{LittleEndian, ReadBytesExt};
use channel::{AcquisitionChannel, ChannelIdentifier};
use images::read_image_data;

use std::fs::File;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};

use std::collections::HashMap;
use std::io::SeekFrom;
use std::io::{prelude::*, BufReader};

use error::MCDError;
use mcd::{MCDParser, ParserState};

use nalgebra::Vector2;
use transform::AffineTransform;

use image::io::Reader as ImageReader;
use image::{DynamicImage, ImageFormat, RgbaImage};
use std::io::Cursor;

const BUF_SIZE: usize = 4096;

/// Print to `writer` trait
pub trait Print {
    /// Formats and prints to `writer`
    fn print<W: fmt::Write + ?Sized>(&self, writer: &mut W, indent: usize) -> fmt::Result;
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

#[derive(Debug)]
pub struct BoundingBox {
    pub min_x: f64,
    pub min_y: f64,
    pub width: f64,
    pub height: f64,
}

pub struct ChannelImage {
    width: i32,
    height: i32,
    range: (f32, f32),
    valid_pixels: usize,
    data: Vec<f32>,
}

#[cfg(test)]
mod tests {
    use super::*;

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

        let slide = mcd.slide(1).unwrap();

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
        let path = std::path::Path::new(output_location);

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

        let resized_image = slide.create_overview_image(15000);
        resized_image
            .save(path.join("slide_overview.jpeg"))
            .unwrap();

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
