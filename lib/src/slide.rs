use core::fmt;
use std::{
    collections::HashMap,
    io::{Cursor, Read, Seek},
    sync::{Arc, Mutex},
};

use image::{
    imageops::FilterType, DynamicImage, ImageBuffer, ImageFormat, RgbImage, Rgba, RgbaImage,
};
use image::{io::Reader as ImageReader, Pixel};

use crate::{channel::ChannelIdentifier, images::read_image_data, Panorama, Print};

use crate::mcd::SlideXML;

#[derive(Debug)]
pub struct Slide<T: Seek + Read> {
    pub(crate) reader: Option<Arc<Mutex<T>>>,

    id: u16,
    uid: String,
    description: String,
    filename: String,
    slide_type: String,
    width_um: f64,
    height_um: f64,

    image_start_offset: i64,
    image_end_offset: i64,
    image_file: String,

    sw_version: String,

    panoramas: HashMap<u16, Panorama<T>>,
}

impl<T: Seek + Read> From<SlideXML> for Slide<T> {
    fn from(slide: SlideXML) -> Self {
        Slide {
            reader: None,

            id: slide.id.unwrap(),
            uid: slide.uid.unwrap(),
            description: slide.description.unwrap(),
            filename: slide.filename.unwrap(),
            slide_type: slide.slide_type.unwrap(),
            width_um: slide.width_um.unwrap(),
            height_um: slide.height_um.unwrap(),
            image_start_offset: slide.image_start_offset.unwrap(),
            image_end_offset: slide.image_end_offset.unwrap(),
            image_file: slide.image_file.unwrap(),
            sw_version: slide.sw_version.unwrap(),

            panoramas: HashMap::new(),
        }
    }
}

impl<T: Seek + Read> Slide<T> {
    pub fn id(&self) -> u16 {
        self.id
    }

    pub fn uid(&self) -> &str {
        &self.uid
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn width_in_um(&self) -> f64 {
        self.width_um
    }

    pub fn height_in_um(&self) -> f64 {
        self.height_um
    }

    pub fn filename(&self) -> &str {
        &self.filename
    }

    pub fn image_file(&self) -> &str {
        &self.image_file
    }

    pub fn software_version(&self) -> &str {
        &self.sw_version
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
        reader.set_format(ImageFormat::Jpeg);
        reader.decode().unwrap()
    }

    pub fn image(&self) -> RgbImage {
        match self.dynamic_image() {
            DynamicImage::ImageRgb8(rgb8) => rgb8,
            _ => panic!("Unexpected DynamicImage type"),
        }
    }

    pub fn create_overview_image(&self, width: u32) -> RgbaImage {
        let slide_image = self.dynamic_image();

        // Move into function to help debugging
        match &slide_image {
            DynamicImage::ImageLuma8(_grey_image) => println!("ImageLuma8"),
            DynamicImage::ImageLumaA8(_grey_alpha_image) => println!("ImageLumaA8"),
            DynamicImage::ImageRgb8(_rgb8) => println!("ImageRgb8"),
            DynamicImage::ImageRgba8(_rgba8) => println!("ImageRgba8"),
            DynamicImage::ImageBgr8(_bgr8) => println!("ImageBgr8"),
            DynamicImage::ImageBgra8(_bgra8) => println!("ImageBgra8"),
            DynamicImage::ImageLuma16(_luma16) => println!("ImageLuma16"),
            DynamicImage::ImageLumaA16(_lumaa16) => println!("ImageLumaA16"),
            DynamicImage::ImageRgb16(_rgb16) => println!("ImageRgb16"),
            DynamicImage::ImageRgba16(_rgba16) => println!("ImageRgba16"),
        }

        println!("Decoded !");

        slide_image.save("slide.jpeg").unwrap();

        println!("Saved !");

        let mut resized_image = slide_image
            .resize_exact(width, width / 3, FilterType::Nearest)
            .to_rgba8();
        println!("Resized !");

        let scale = 75000.0 / width as f64;

        for panorama in self.panoramas() {
            let panorama_image = panorama.image();

            //let panorama_image = panorama_image.to_rgba8();

            let bounding_box = panorama.slide_bounding_box();
            let transform = panorama.to_slide_transform();

            println!("[Panorama] Bounding box = {:?}", bounding_box);
            println!("[Panorama] Transform = {:?}", transform);

            for y in 0..panorama_image.height() {
                for x in 0..panorama_image.width() {
                    let new_point = transform.transform_to_slide(x as f64, y as f64).unwrap();

                    let pixel = *panorama_image.get_pixel(x, y);

                    resized_image.put_pixel(
                        (new_point[0] / scale).round() as u32,
                        ((self.height_um - new_point[1]) / scale).round() as u32,
                        pixel,
                    );
                }
            }

            for acquisition in panorama.acquisitions() {
                let bounding_box = acquisition.slide_bounding_box();
                let transform = acquisition.to_slide_transform();

                println!("[Acquisition] Bounding box = {:?}", bounding_box);
                println!("[Acquisition] Transform = {:?}", transform);

                let data = acquisition.channel_data(ChannelIdentifier::Name("Ir(191)".to_string()));

                let mut index = 0;

                let mut acq_image: ImageBuffer<Rgba<u8>, Vec<u8>> =
                    ImageBuffer::new(data.width as u32, data.height as u32);

                for y in 0..data.height {
                    if index >= data.valid_pixels {
                        break;
                    }

                    for x in 0..data.width {
                        if index >= data.valid_pixels {
                            break;
                        }

                        let new_point = transform.transform_to_slide(x as f64, y as f64).unwrap();

                        let g = ((data.data[index] / 100.0) * 255.0) as u8;
                        let g = g as f64 / 255.0;

                        let cur_pixel = acq_image.get_pixel_mut(x as u32, y as u32).channels_mut();
                        cur_pixel[1] = (g * 255.0) as u8;
                        cur_pixel[3] = 255;

                        //let pixel = Rgba::from_channels(0, g, 0, g);

                        let current_pixel = resized_image
                            .get_pixel_mut(
                                (new_point[0] / scale).round() as u32,
                                ((self.height_um - new_point[1]) / scale).round() as u32,
                            )
                            .channels_mut();

                        let r = (current_pixel[0] as f64 / 255.0) * (1.0 - g);
                        let g = g * g + (current_pixel[1] as f64 / 255.0) * (1.0 - g);
                        let b = (current_pixel[2] as f64 / 255.0) * (1.0 - g);

                        current_pixel[0] = (r * 255.0) as u8;
                        current_pixel[1] = (g * 255.0) as u8;
                        current_pixel[2] = (b * 255.0) as u8;

                        index += 1;
                    }
                }

                acq_image
                    .save(format!("{}_Ir(191).png", acquisition.description()))
                    .unwrap();

                println!("Finished reading data");
            }
        }

        resized_image
    }

    pub fn panorama_ids(&self) -> Vec<u16> {
        let mut ids: Vec<u16> = Vec::with_capacity(self.panoramas.len());

        for id in self.panoramas.keys() {
            ids.push(*id);
        }

        ids.sort_unstable();

        ids
    }

    pub fn panorama(&self, id: u16) -> Option<&Panorama<T>> {
        self.panoramas.get(&id)
    }

    // Get panoramas ordered by ID
    pub fn panoramas(&self) -> Vec<&Panorama<T>> {
        let mut panoramas = Vec::new();

        let ids = self.panorama_ids();
        for id in ids {
            panoramas.push(
                self.panorama(id)
                    .expect("Should only be getting panoramas that exist"),
            );
        }

        panoramas
    }

    pub(crate) fn panoramas_mut(&mut self) -> &mut HashMap<u16, Panorama<T>> {
        &mut self.panoramas
    }
}

impl<T: Seek + Read> Print for Slide<T> {
    fn print<W: fmt::Write + ?Sized>(&self, writer: &mut W, indent: usize) -> fmt::Result {
        write!(writer, "{:indent$}", "", indent = indent)?;
        writeln!(writer, "{:-^1$}", "Slide", 36)?;
        writeln!(
            writer,
            "{:indent$}{: <16} | {}",
            "",
            "ID",
            self.id,
            indent = indent
        )?;
        writeln!(
            writer,
            "{:indent$}{: <16} | {}",
            "",
            "UID",
            self.uid,
            indent = indent
        )?;
        writeln!(
            writer,
            "{:indent$}{: <16} | {}",
            "",
            "Description",
            self.description,
            indent = indent
        )?;
        writeln!(
            writer,
            "{:indent$}{: <16} | {}",
            "",
            "Filename",
            self.filename,
            indent = indent
        )?;
        writeln!(
            writer,
            "{:indent$}{: <16} | {}",
            "",
            "Type",
            self.slide_type,
            indent = indent
        )?;
        writeln!(
            writer,
            "{:indent$}{: <16} | {} μm x {} μm ",
            "",
            "Dimensions",
            self.width_um,
            self.height_um,
            indent = indent
        )?;
        writeln!(
            writer,
            "{:indent$}{: <16} | {}",
            "",
            "Image File",
            self.image_file,
            indent = indent
        )?;
        writeln!(
            writer,
            "{:indent$}{: <16} | {}",
            "",
            "Software Version",
            self.sw_version,
            indent = indent
        )?;

        write!(writer, "{:indent$}", "", indent = indent)?;
        writeln!(writer, "{:-^1$}", "", 36)?;

        writeln!(
            writer,
            "{:indent$}{} panorama(s) with ids: {:?}",
            "",
            self.panoramas.len(),
            self.panorama_ids(),
            indent = indent + 1
        )?;
        write!(writer, "{:indent$}", "", indent = indent)?;
        writeln!(writer, "{:-^1$}", "", 36)?;

        Ok(())
    }
}

impl<T: Seek + Read> fmt::Display for Slide<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.print(f, 0)
    }
}
