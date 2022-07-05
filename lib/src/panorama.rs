use core::fmt;
use std::{
    collections::HashMap,
    io::{BufRead, Seek},
    sync::{Arc, Mutex},
};

use image::ImageFormat;
use nalgebra::Vector2;

use crate::{
    mcd::PanoramaXML, transform::AffineTransform, Acquisition, BoundingBox, OnSlide, OpticalImage,
    Print,
};

#[derive(Debug)]
pub enum PanoramaType {
    Default,
    Imported,
    Instrument,
}

/// Represents a panorama (containing one or more acquisitions)
#[derive(Debug)]
pub struct Panorama<T: Seek + BufRead> {
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

    panorama_type: Option<PanoramaType>,
    is_locked: Option<bool>,
    rotation_angle: Option<u16>,

    acquisitions: HashMap<u16, Acquisition<T>>,
}

impl<T: Seek + BufRead> Panorama<T> {
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

    /// Returns a scaling coefficient for pixel sizes
    pub fn pixel_scale_coef(&self) -> f64 {
        self.pixel_scale_coef
    }

    /// Returns the type of the panorama image, if known. This is unknown in the first version of the schema
    pub fn panorama_type(&self) -> Option<&PanoramaType> {
        self.panorama_type.as_ref()
    }

    /// Returns whether the panorama is locked or not (if known). This is unknown in the first version of the schema
    pub fn is_locked(&self) -> Option<bool> {
        self.is_locked
    }

    /// Returns the rotation angle of the panorama (if known). This is unknown in the first version of the schema
    pub fn rotation_angle(&self) -> Option<u16> {
        self.rotation_angle
    }

    /// Returns a sorted (acsending) list of acquisition IDs
    pub fn acquisition_ids(&self) -> Vec<u16> {
        let mut ids: Vec<u16> = Vec::with_capacity(self.acquisitions.len());

        for id in self.acquisitions.keys() {
            ids.push(*id);
        }

        ids.sort_unstable();

        ids
    }

    /// Returns an acquisition with the supplied ID, or None if none exists
    pub fn acquisition(&self, id: u16) -> Option<&Acquisition<T>> {
        self.acquisitions.get(&id)
    }

    /// Returns a vector of acquisition references ordered by acquistion ID number
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

    pub(crate) fn fix_image_dimensions(&mut self) {
        if self.has_image() && (self.pixel_width == 0 || self.pixel_height == 0) {
            let image = self.image().unwrap();
            let dims = image.dimensions().unwrap();

            self.pixel_width = dims.0 as i64;
            self.pixel_height = dims.1 as i64;
        }
    }

    /// Returns true if an image is associated with this panorama
    pub fn has_image(&self) -> bool {
        (self.image_end_offset - self.image_start_offset) > 0
    }

    /// Returns the optical image
    pub fn image(&self) -> Option<OpticalImage<T>> {
        if self.has_image() {
            Some(OpticalImage {
                reader: self.reader.as_ref()?.clone(),
                start_offset: self.image_start_offset,
                end_offset: self.image_end_offset,
                image_format: self.image_format,
            })
        } else {
            None
        }
    }
}

/*
impl<T: Seek + Read> OpticalImage for Panorama<T> {
    fn has_image(&self) -> bool {
        (self.image_end_offset - self.image_start_offset) > 0
    }

    /// Returns the format that the panorama image is stored in
    fn image_format(&self) -> ImageFormat {
        self.image_format
    }

    /// Returns the binary data for the image, exactly as stored in the .mcd file
    fn image_data(&self) -> Result<Vec<u8>, std::io::Error> {
        let mutex = self
            .reader
            .as_ref()
            .expect("Should have copied the reader across");
        let reader = mutex.lock().unwrap();

        read_image_data(reader, self.image_start_offset, self.image_end_offset)
    }

    /// Returns a decoded RgbaImage of the panorama image
    fn image(&self) -> RgbaImage {
        match self.dynamic_image() {
            DynamicImage::ImageRgba8(rgba8) => rgba8,
            _ => panic!("Unexpected DynamicImage type"),
        }
    }
} */

impl<T: Seek + BufRead> OnSlide for Panorama<T> {
    /// Returns the bounding box encompasing the panorama image area on the slide (in μm)
    fn slide_bounding_box(&self) -> BoundingBox<f64> {
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

    /// Returns the affine transformation from pixel coordinates within the panorama to to the slide coordinates (μm)
    fn to_slide_transform(&self) -> AffineTransform<f64> {
        if !self.has_image() {
            return AffineTransform::identity();
        }

        let mut moving_points = Vec::new();
        let mut fixed_points = Vec::new();

        moving_points.push(Vector2::new(self.slide_x1_pos_um, self.slide_y1_pos_um));
        moving_points.push(Vector2::new(self.slide_x2_pos_um, self.slide_y2_pos_um));
        moving_points.push(Vector2::new(self.slide_x3_pos_um, self.slide_y3_pos_um));
        //moving_points.push(Vector2::new(self.slide_x4_pos_um, self.slide_y4_pos_um));

        // println!(
        //     "slide {} {} {}",
        //     self.slide_y1_pos_um, self.slide_y2_pos_um, self.slide_y3_pos_um
        // );

        fixed_points.push(Vector2::new(0.0, self.pixel_height as f64));
        fixed_points.push(Vector2::new(
            self.pixel_width as f64,
            self.pixel_height as f64,
        ));
        fixed_points.push(Vector2::new(self.pixel_width as f64, 0.0));
        //fixed_points.push(Vector2::new(0.0, self.pixel_height as f64));

        AffineTransform::from_points(moving_points, fixed_points)
    }
}

#[rustfmt::skip]
impl<T: Seek + BufRead> Print for Panorama<T> {
    fn print<W: fmt::Write + ?Sized>(&self, writer: &mut W, indent: usize) -> fmt::Result {
        write!(writer, "{:indent$}", "", indent = indent)?;
        writeln!(writer, "{:-^1$}", "Panorama", 42)?;

        writeln!(writer, "{:indent$}{: <20} | {}", "", "ID", self.id, indent = indent)?;
        writeln!(writer, "{:indent$}{: <20} | {}", "", "Slide ID", self.slide_id, indent = indent)?;
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

impl<T: Seek + BufRead + 'static> fmt::Display for Panorama<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.print(f, 0)
    }
}

impl<T: Seek + BufRead> From<PanoramaXML> for Panorama<T> {
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

            panorama_type: panorama.panorama_type,
            is_locked: panorama.is_locked,
            rotation_angle: panorama.rotation_angle,

            acquisitions: HashMap::new(),
        }
    }
}
