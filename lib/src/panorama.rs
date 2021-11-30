use core::fmt;
use std::{
    collections::HashMap,
    io::{Cursor, Read, Seek},
    sync::{Arc, Mutex},
};

use image::io::Reader as ImageReader;
use image::{DynamicImage, ImageFormat, RgbaImage};
use nalgebra::Vector2;

use crate::{
    images::read_image_data, mcd::PanoramaXML, transform::AffineTransform, Acquisition,
    BoundingBox, Print,
};

/// Represents a panorama (containing one or more acquisitions)
#[derive(Debug)]
pub struct Panorama<T: Seek + Read> {
    pub(crate) reader: Option<Arc<Mutex<T>>>,

    id: u16,
    slide_id: u16,
    description: String,
    slide_x1_pos_um: f64,
    slide_y1_pos_um: f64,
    slide_x2_pos_um: f64,
    slide_y2_pos_um: f64,
    slide_x3_pos_um: f64,
    slide_y3_pos_um: f64,
    slide_x4_pos_um: f64,
    slide_y4_pos_um: f64,

    image_start_offset: i64,
    image_end_offset: i64,
    pixel_width: i64,
    pixel_height: i64,
    image_format: ImageFormat,
    pixel_scale_coef: f64,

    acquisitions: HashMap<u16, Acquisition<T>>,
}

impl<T: Seek + Read> Panorama<T> {
    /// Returns the panorama ID
    pub fn id(&self) -> u16 {
        self.id
    }

    /// Returns the slide ID to which this panorama belongs
    pub fn slide_id(&self) -> u16 {
        self.slide_id
    }

    /// Returns the given description for the panorama
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Return the dimensions in pixels (width, height) of the panorama image
    pub fn dimensions(&self) -> (i64, i64) {
        (self.pixel_width, self.pixel_height)
    }

    pub fn pixel_scale_coef(&self) -> f64 {
        self.pixel_scale_coef
    }

    pub fn image_format(&self) -> ImageFormat {
        self.image_format
    }

    pub fn image_data(&self) -> Result<Vec<u8>, std::io::Error> {
        let mutex = self
            .reader
            .as_ref()
            .expect("Should have copied the reader across");
        let reader = mutex.lock().unwrap();

        read_image_data(reader, self.image_start_offset, self.image_end_offset)
    }

    fn dynamic_image(&self) -> DynamicImage {
        let mut reader = ImageReader::new(Cursor::new(self.image_data().unwrap()));
        reader.set_format(ImageFormat::Png);
        reader.decode().unwrap()
    }

    pub fn image(&self) -> RgbaImage {
        match self.dynamic_image() {
            DynamicImage::ImageRgba8(rgba8) => rgba8,
            _ => panic!("Unexpected DynamicImage type"),
        }
    }

    pub fn slide_bounding_box(&self) -> BoundingBox {
        let min_x = f64::min(
            self.slide_x1_pos_um,
            f64::min(
                self.slide_x2_pos_um,
                f64::min(self.slide_x3_pos_um, self.slide_x4_pos_um),
            ),
        );
        let min_y = f64::min(
            self.slide_y1_pos_um,
            f64::min(
                self.slide_y2_pos_um,
                f64::min(self.slide_y3_pos_um, self.slide_y4_pos_um),
            ),
        );
        let max_x = f64::max(
            self.slide_x1_pos_um,
            f64::max(
                self.slide_x2_pos_um,
                f64::max(self.slide_x3_pos_um, self.slide_x4_pos_um),
            ),
        );
        let max_y = f64::max(
            self.slide_y1_pos_um,
            f64::max(
                self.slide_y2_pos_um,
                f64::max(self.slide_y3_pos_um, self.slide_y4_pos_um),
            ),
        );

        BoundingBox {
            min_x,
            min_y,
            width: (max_x - min_x).abs(),
            height: (max_y - min_y).abs(),
        }
    }

    pub fn acquisition_ids(&self) -> Vec<u16> {
        let mut ids: Vec<u16> = Vec::with_capacity(self.acquisitions.len());

        for id in self.acquisitions.keys() {
            ids.push(*id);
        }

        ids.sort_unstable();

        ids
    }

    pub fn acquisition(&self, id: u16) -> Option<&Acquisition<T>> {
        self.acquisitions.get(&id)
    }

    fn acquisition_mut(&mut self, id: u16) -> Option<&mut Acquisition<T>> {
        self.acquisitions.get_mut(&id)
    }

    // Get acquisitions ordered by ID
    pub fn acquisitions(&self) -> Vec<&Acquisition<T>> {
        let mut acquisitions = Vec::new();

        let ids = self.acquisition_ids();
        for id in ids {
            acquisitions.push(
                self.acquisition(id)
                    .expect("Should only be getting acquisitions that exist"),
            );
        }

        acquisitions
    }

    pub(crate) fn acquisitions_mut(&mut self) -> &mut HashMap<u16, Acquisition<T>> {
        &mut self.acquisitions
    }

    pub fn to_slide_transform(&self) -> AffineTransform<f64> {
        let mut moving_points = Vec::new();
        let mut fixed_points = Vec::new();

        moving_points.push(Vector2::new(self.slide_x1_pos_um, self.slide_y1_pos_um));
        moving_points.push(Vector2::new(self.slide_x2_pos_um, self.slide_y2_pos_um));
        moving_points.push(Vector2::new(self.slide_x3_pos_um, self.slide_y3_pos_um));
        //moving_points.push(Vector2::new(self.slide_x4_pos_um, self.slide_y4_pos_um));

        fixed_points.push(Vector2::new(0.0, 0.0));
        fixed_points.push(Vector2::new(self.pixel_width as f64, 0.0));
        fixed_points.push(Vector2::new(
            self.pixel_width as f64,
            self.pixel_height as f64,
        ));
        //fixed_points.push(Vector2::new(0.0, self.pixel_height as f64));

        AffineTransform::from_points(moving_points, fixed_points)
    }
}

impl<T: Seek + Read> Print for Panorama<T> {
    fn print<W: fmt::Write + ?Sized>(&self, writer: &mut W, indent: usize) -> fmt::Result {
        write!(writer, "{:indent$}", "", indent = indent)?;
        writeln!(writer, "{:-^1$}", "Panorama", 42)?;
        writeln!(
            writer,
            "{:indent$}{: <20} | {}",
            "",
            "ID",
            self.id,
            indent = indent
        )?;
        writeln!(
            writer,
            "{:indent$}{: <20} | {}",
            "",
            "Slide ID",
            self.slide_id,
            indent = indent
        )?;
        writeln!(
            writer,
            "{:indent$}{: <20} | {}",
            "",
            "Description",
            self.description,
            indent = indent
        )?;
        writeln!(
            writer,
            "{:indent$}{: <20} | ({:.4} μm, {:.4} μm)",
            "",
            "Slide coordinates",
            self.slide_x1_pos_um,
            self.slide_y1_pos_um,
            indent = indent
        )?;
        writeln!(
            writer,
            "{:indent$}{: <20} | ({:.4} μm, {:.4} μm)",
            "",
            "",
            self.slide_x2_pos_um,
            self.slide_y2_pos_um,
            indent = indent
        )?;
        writeln!(
            writer,
            "{:indent$}{: <20} | ({:.4} μm, {:.4} μm)",
            "",
            "",
            self.slide_x3_pos_um,
            self.slide_y3_pos_um,
            indent = indent
        )?;
        writeln!(
            writer,
            "{:indent$}{: <20} | ({:.4} μm, {:.4} μm)",
            "",
            "",
            self.slide_x4_pos_um,
            self.slide_y4_pos_um,
            indent = indent
        )?;
        writeln!(
            writer,
            "{:indent$}{: <20} | {} x {}",
            "",
            "Dimensions (pixels)",
            self.pixel_width,
            self.pixel_height,
            indent = indent
        )?;
        writeln!(
            writer,
            "{:indent$}{: <20} | {}",
            "",
            "Pixel scale coef",
            self.pixel_scale_coef,
            indent = indent
        )?;

        write!(writer, "{:indent$}", "", indent = indent)?;
        writeln!(writer, "{:-^1$}", "", 42)?;

        writeln!(
            writer,
            "{} acquisition(s) with ids: {:?}",
            self.acquisitions.len(),
            self.acquisition_ids()
        )?;
        write!(writer, "{:indent$}", "", indent = indent)?;
        writeln!(writer, "{:-^1$}", "", 42)?;

        Ok(())
    }
}

impl<T: Seek + Read + 'static> fmt::Display for Panorama<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.print(f, 0)
    }
}

impl<T: Seek + Read> From<PanoramaXML> for Panorama<T> {
    fn from(panorama: PanoramaXML) -> Self {
        Panorama {
            reader: None,

            id: panorama.id.unwrap(),
            slide_id: panorama.slide_id.unwrap(),
            description: panorama.description.unwrap(),
            slide_x1_pos_um: panorama.slide_x1_pos_um.unwrap(),
            slide_y1_pos_um: panorama.slide_y1_pos_um.unwrap(),
            slide_x2_pos_um: panorama.slide_x2_pos_um.unwrap(),
            slide_y2_pos_um: panorama.slide_y2_pos_um.unwrap(),
            slide_x3_pos_um: panorama.slide_x3_pos_um.unwrap(),
            slide_y3_pos_um: panorama.slide_y3_pos_um.unwrap(),
            slide_x4_pos_um: panorama.slide_x4_pos_um.unwrap(),
            slide_y4_pos_um: panorama.slide_y4_pos_um.unwrap(),
            image_start_offset: panorama.image_start_offset.unwrap(),
            image_end_offset: panorama.image_end_offset.unwrap(),
            pixel_width: panorama.pixel_width.unwrap(),
            pixel_height: panorama.pixel_height.unwrap(),
            image_format: panorama.image_format.unwrap(),
            pixel_scale_coef: panorama.pixel_scale_coef.unwrap(),

            acquisitions: HashMap::new(),
        }
    }
}
