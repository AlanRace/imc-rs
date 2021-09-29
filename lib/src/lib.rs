mod parser;
mod transform;

use std::fmt;

use std::sync::{Arc, Mutex};

use std::collections::HashMap;
use std::io::prelude::*;
use std::io::SeekFrom;

use parser::{MCDParser, ParserState};

use nalgebra::Vector2;
use transform::AffineTransform;

use std::convert::TryInto;

const BUF_SIZE: usize = 4096;

fn find_mcd_start(chunk: &std::vec::Vec<u8>, chunk_size: usize) -> usize {
    for start_index in 0..chunk_size {
        match std::str::from_utf8(&chunk[start_index..]) {
            Ok(_data) => {
                return start_index - 1;
            }
            Err(_) => {}
        }
    }

    0
}

fn u16_from_u8(a: &mut [u16], v: &[u8]) {
    for i in 0..a.len() {
        a[i] = (v[i * 2] as u16) | ((v[i * 2 + 1] as u16) << 8)
    }
}

fn get_image_data<T: Read + Seek>(
    mut source: std::sync::MutexGuard<T>,
    start_offset: i64,
    end_offset: i64,
) -> std::io::Result<Vec<u8>> {
    let mut image_start_offset = start_offset;

    // Add an offset to skip the C# Drawing data
    image_start_offset += 161;
    let image_size = end_offset - image_start_offset;

    let mut buf_u8 = vec![0; image_size.try_into().unwrap()];

    match source.seek(SeekFrom::Start(image_start_offset as u64)) {
        Ok(_seek) => match source.read_exact(&mut buf_u8) {
            Ok(()) => Ok(buf_u8),
            Err(error) => Err(error),
        },
        Err(error) => Err(error),
    }
}

//pub trait SeekRead: Seek + Read {}
pub trait Print {
    fn print<W: fmt::Write + ?Sized>(&self, writer: &mut W, indent: usize) -> fmt::Result;
}

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

    /*pub fn get_reader(&self) -> &T {
        &self.reader
    }*/

    pub fn get_reader(&self) -> &Arc<Mutex<T>> {
        &self.reader
    }

    pub fn get_slide_ids(&self) -> Vec<u16> {
        let mut ids: Vec<u16> = Vec::with_capacity(self.slides.len());

        for (id, _panorama) in &self.slides {
            ids.push(*id);
        }

        ids.sort();

        ids
    }

    pub fn get_slide(&self, id: &u16) -> Option<&Slide<T>> {
        self.slides.get(id)
    }

    pub fn get_channels(&self) -> Vec<&AcquisitionChannel> {
        let mut channels = HashMap::new();

        // This should be unnecessary - hopefully there is only one set of channels per dataset?
        for (_, slide) in &self.slides {
            for panorama in slide.get_panoramas() {
                for acquisition in panorama.get_acquisitions() {
                    for channel in acquisition.get_channels() {
                        if !channels.contains_key(channel.get_name()) {
                            channels.insert(channel.get_name(), channel);
                        }
                    }
                }
            }
        }

        let mut ordered_channels = Vec::new();
        for (_, channel) in channels.drain() {
            ordered_channels.push(channel);
        }

        ordered_channels.sort_by(|a, b| a.order_number.cmp(&b.order_number));

        ordered_channels
    }

    pub fn parse(reader: T, location: &str) -> Self {
        //let mut file = File::open(filename).unwrap();
        let mcd = MCD::new(reader, location);
        let mut parser = MCDParser::new(mcd);

        let chunk_size: i64 = 1000;
        let mut cur_offset: i64 = 0;

        //let mut strings = Vec::<String>::new();

        let mut buf_u8 = vec![0; chunk_size.try_into().unwrap()];

        loop {
            let mut reader = parser.current_mcd.as_mut().unwrap().reader.lock().unwrap();

            reader
                .seek(SeekFrom::End(-cur_offset - chunk_size))
                .unwrap();

            //let mut buf = String::new();
            reader.read_exact(&mut buf_u8).unwrap();
            // .read_to_string(&mut buf).unwrap();

            match std::str::from_utf8(&buf_u8) {
                Ok(_data) => {} //strings.push(data.to_owned()),
                Err(_error) => {
                    // Found the final chunk, so find the start point
                    let start_index = find_mcd_start(&buf_u8, chunk_size.try_into().unwrap());

                    let total_size = cur_offset + chunk_size - (start_index as i64);
                    buf_u8 = vec![0; total_size.try_into().unwrap()];

                    reader.seek(SeekFrom::End(-total_size)).unwrap();
                    reader.read_exact(&mut buf_u8).unwrap();

                    //println!("Start Index: {}", start_index);

                    //strings.push(data.to_owned());
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

        /*for s in strings.into_iter().rev() {
            let mut buf_u16: Vec<u16> = vec![0; (chunk_size/2).try_into().unwrap()];
            u16_from_u8(&mut buf_u16, s.as_ref());

            match String::from_utf16(&buf_u16) {
                Ok(data) => combined_xml.push_str(&data),
                Err(error) => {
                    println!("{}", error)
                }
            }
        }*/

        //println!("{:?}", &combined_xml);

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

        parser.get_mcd()
    }
    /*pub(crate) fn add_acquisition_channel(&mut self, channel: AcquisitionChannel) {
            let acquisition_id = channel.acquisition_id.expect("Must have an AcquisitionID or won't know which Acquisition the AcquisitionChannel belongs to");
    println!("{:?}", self.acquisitions);


            panic!("No acquistion with ID: {:?}", acquisition_id);
        }*/
}

impl<T: Seek + Read> Print for MCD<T> {
    fn print<W: fmt::Write + ?Sized>(&self, writer: &mut W, indent: usize) -> fmt::Result {
        writeln!(writer, "MCD File: {}", self.location)?;

        match self.xmlns.as_ref() {
            Some(xmlns) => writeln!(writer, "XML Namespace: {}", xmlns)?,
            None => {}
        }

        for (_id, slide) in &self.slides {
            slide.print(writer, indent + 1)?;
        }

        let channels = self.get_channels();
        write!(writer, "{:indent$}", "", indent = indent)?;
        writeln!(writer, "{:-^1$}", "Channels", 25)?;
        for channel in channels {
            writeln!(
                writer,
                "{0: <2} | {1: <10} | {2: <10}",
                channel.get_order_number(),
                channel.get_name(),
                channel.get_label()
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

pub struct MCDPublic {}

#[derive(Debug)]
pub struct Slide<T: Seek + Read> {
    reader: Option<Arc<Mutex<T>>>,

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

impl<T: Seek + Read> Slide<T> {
    pub fn get_id(&self) -> u16 {
        self.id
    }

    pub fn get_image(&self) -> Result<Vec<u8>, std::io::Error> {
        let mutex = self
            .reader
            .as_ref()
            .expect("Should have copied the reader across");
        let reader = mutex.lock().unwrap();

        get_image_data(
            reader,
            self.image_start_offset,
            self.image_end_offset,
        )
    }


    pub fn get_panorama_ids(&self) -> Vec<u16> {
        let mut ids: Vec<u16> = Vec::with_capacity(self.panoramas.len());

        for (id, _panorama) in &self.panoramas {
            ids.push(*id);
        }

        ids.sort();

        ids
    }

    pub fn get_panorama(&self, id: &u16) -> Option<&Panorama<T>> {
        self.panoramas.get(id)
    }

    // Get panoramas ordered by ID
    pub fn get_panoramas(&self) -> Vec<&Panorama<T>> {
        let mut panoramas = Vec::new();

        let ids = self.get_panorama_ids();
        for id in ids {
            panoramas.push(
                self.get_panorama(&id)
                    .expect("Should only be getting panoramas that exist"),
            );
        }

        panoramas
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
            self.get_panorama_ids(),
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

#[derive(Debug)]
pub enum ImageFormat {
    Png,
}

#[derive(Debug)]
pub struct Panorama<T: Seek + Read> {
    reader: Option<Arc<Mutex<T>>>,

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
    pub fn get_id(&self) -> u16 {
        self.id
    }

    pub fn get_image(&self) -> Result<Vec<u8>, std::io::Error> {
        let mutex = self
            .reader
            .as_ref()
            .expect("Should have copied the reader across");
        let reader = mutex.lock().unwrap();

        get_image_data(reader, self.image_start_offset, self.image_end_offset)
    }

    pub fn slide_bounding_box(&self) -> BoundingBox {
        let min_x = f64::min(self.slide_x1_pos_um, f64::min(self.slide_x2_pos_um, f64::min(self.slide_x3_pos_um, self.slide_x4_pos_um)));
        let min_y = f64::min(self.slide_y1_pos_um, f64::min(self.slide_y2_pos_um, f64::min(self.slide_y3_pos_um, self.slide_y4_pos_um)));
        let max_x = f64::max(self.slide_x1_pos_um, f64::max(self.slide_x2_pos_um, f64::max(self.slide_x3_pos_um, self.slide_x4_pos_um)));
        let max_y = f64::max(self.slide_y1_pos_um, f64::max(self.slide_y2_pos_um, f64::max(self.slide_y3_pos_um, self.slide_y4_pos_um)));

        BoundingBox {
            min_x,
            min_y,
            width: (max_x - min_x).abs(),
            height: (max_y - min_y).abs(),
        }
    }

    pub fn get_acquisition_ids(&self) -> Vec<u16> {
        let mut ids: Vec<u16> = Vec::with_capacity(self.acquisitions.len());

        for (id, _acquisition) in &self.acquisitions {
            ids.push(*id);
        }

        ids.sort();

        ids
    }

    pub fn get_acquisition(&self, id: &u16) -> Option<&Acquisition<T>> {
        self.acquisitions.get(id)
    }

    // Get acquisitions ordered by ID
    pub fn get_acquisitions(&self) -> Vec<&Acquisition<T>> {
        let mut acquisitions = Vec::new();

        let ids = self.get_acquisition_ids();
        for id in ids {
            acquisitions.push(
                self.get_acquisition(&id)
                    .expect("Should only be getting acquisitions that exist"),
            );
        }

        acquisitions
    }

    pub fn to_slide_transform(&self) -> AffineTransform<f64> {
        let mut moving_points = Vec::new();
        let mut fixed_points = Vec::new();

        moving_points.push(Vector2::new(self.slide_x1_pos_um, self.slide_y1_pos_um));
        moving_points.push(Vector2::new(self.slide_x2_pos_um, self.slide_y2_pos_um));
        moving_points.push(Vector2::new(self.slide_x3_pos_um, self.slide_y3_pos_um));
        //moving_points.push(Vector2::new(slide_x4_pos_um, self.slide_y4_pos_um));

        fixed_points.push(Vector2::new(0.0, 0.0));
        fixed_points.push(Vector2::new(self.pixel_width as f64, 0.0));
        fixed_points.push(Vector2::new(0.0, self.pixel_height as f64));
        //fixed_points.push(Vector2::new(self.pixel_width as f64, self.pixel_height as f64));

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
            self.get_acquisition_ids()
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

#[derive(Debug)]
pub struct AcquisitionChannel {
    id: u16,
    channel_name: String,
    order_number: i16,
    acquisition_id: u16,
    channel_label: String,
}

impl AcquisitionChannel {
    fn get_id(&self) -> u16 {
        self.id
    }
    fn get_name(&self) -> &str {
        &self.channel_name
    }
    fn get_order_number(&self) -> i16 {
        self.order_number
    }
    fn get_label(&self) -> &str {
        &self.channel_label
    }
}

#[derive(Debug)]
enum DataFormat {
    Float,
}

#[derive(Debug)]
pub struct Acquisition<T: Read + Seek> {
    reader: Option<Arc<Mutex<T>>>,

    id: u16,
    description: String,
    ablation_power: f64,
    ablation_distance_between_shots_x: f64,
    ablation_distance_between_shots_y: f64,
    ablation_frequency: f64,
    acquisition_roi_id: i16,
    order_number: i16,
    signal_type: String,
    dual_count_start: String,
    data_start_offset: i64,
    data_end_offset: i64,
    start_timestamp: String,
    end_timestamp: String,
    after_ablation_image_start_offset: i64,
    after_ablation_image_end_offset: i64,
    before_ablation_image_start_offset: i64,
    before_ablation_image_end_offset: i64,
    roi_start_x_pos_um: f64,
    roi_start_y_pos_um: f64,
    roi_end_x_pos_um: f64,
    roi_end_y_pos_um: f64,
    movement_type: String,
    segment_data_format: DataFormat,
    value_bytes: u8,
    max_x: i32,
    max_y: i32,
    plume_start: i32,
    plume_end: i32,
    template: String,

    channels: Vec<AcquisitionChannel>,
}

#[derive(Debug)]
pub struct BoundingBox {
    min_x: f64,
    min_y: f64,
    width: f64,
    height: f64,
}

impl<T: Read + Seek> Acquisition<T> {
    pub fn to_slide_transform(&self) -> AffineTransform<f64> {
        let mut moving_points = Vec::new();
        let mut fixed_points = Vec::new();

        // There seems to be a bug where the start and end x pos is recorded as the same value
        let roi_end_x_pos_um = match self.roi_start_x_pos_um == self.roi_end_x_pos_um {
            true => {
                self.roi_start_x_pos_um
                    + (self.max_x as f64 * self.ablation_distance_between_shots_x)
            }
            false => self.roi_end_x_pos_um,
        };

        moving_points.push(Vector2::new(
            self.roi_start_x_pos_um,
            self.roi_start_y_pos_um,
        ));
        moving_points.push(Vector2::new(roi_end_x_pos_um, self.roi_start_y_pos_um));
        moving_points.push(Vector2::new(self.roi_start_x_pos_um, self.roi_end_y_pos_um));
        //moving_points.push(Vector2::new(roi_end_x_pos_um, self.roi_end_y_pos_um));

        fixed_points.push(Vector2::new(0.0, 0.0));
        fixed_points.push(Vector2::new(self.max_x as f64, 0.0));
        fixed_points.push(Vector2::new(0.0, self.max_y as f64));
        //fixed_points.push(Vector2::new(self.max_x as f64, self.max_y as f64));

        AffineTransform::from_points(moving_points, fixed_points)
    }

    pub fn slide_bounding_box(&self) -> BoundingBox {
        // There seems to be a bug where the start and end x pos is recorded as the same value
        let roi_end_x_pos_um = match self.roi_start_x_pos_um == self.roi_end_x_pos_um {
            true => {
                self.roi_start_x_pos_um
                    + (self.max_x as f64 * self.ablation_distance_between_shots_x)
            }
            false => self.roi_end_x_pos_um,
        };

        BoundingBox {
            min_x: f64::min(self.roi_start_x_pos_um, roi_end_x_pos_um),
            min_y: f64::min(self.roi_start_y_pos_um, self.roi_end_y_pos_um),
            width: (roi_end_x_pos_um - self.roi_start_x_pos_um).abs(),
            height: (self.roi_end_y_pos_um - self.roi_start_y_pos_um).abs(),
        }
    }

    pub fn get_before_ablation_image(&self) -> Result<Vec<u8>, std::io::Error> {
        let mutex = self
            .reader
            .as_ref()
            .expect("Should have copied the reader across");
        let reader = mutex.lock().unwrap();

        get_image_data(
            reader,
            self.before_ablation_image_start_offset,
            self.before_ablation_image_end_offset,
        )
    }

    pub fn get_after_ablation_image(&self) -> Result<Vec<u8>, std::io::Error> {
        let mutex = self
            .reader
            .as_ref()
            .expect("Should have copied the reader across");
        let reader = mutex.lock().unwrap();

        get_image_data(
            reader,
            self.after_ablation_image_start_offset,
            self.after_ablation_image_end_offset,
        )
    }

    pub fn get_channels(&self) -> &Vec<AcquisitionChannel> {
        &self.channels
    }
}

impl<T: Seek + Read> Print for Acquisition<T> {
    fn print<W: fmt::Write + ?Sized>(&self, writer: &mut W, indent: usize) -> fmt::Result {
        write!(writer, "{:indent$}", "", indent = indent)?;
        writeln!(writer, "{:-^1$}", "Acquisition", 48)?;
        writeln!(
            writer,
            "{:indent$}{: <22} | {}",
            "",
            "ID",
            self.id,
            indent = indent
        )?;
        writeln!(
            writer,
            "{:indent$}{: <22} | {}",
            "",
            "Description",
            self.description,
            indent = indent
        )?;
        writeln!(
            writer,
            "{:indent$}{: <22} | {}",
            "",
            "Order number",
            self.order_number,
            indent = indent
        )?;
        writeln!(
            writer,
            "{:indent$}{: <22} | {} x {}",
            "",
            "Dimensions (pixels)",
            self.max_x,
            self.max_y,
            indent = indent
        )?;
        writeln!(
            writer,
            "{:indent$}{: <22} | {} x {}",
            "",
            "Distance between shots",
            self.ablation_distance_between_shots_x,
            self.ablation_distance_between_shots_y,
            indent = indent
        )?;
        writeln!(
            writer,
            "{:indent$}{: <22} | {}",
            "",
            "Signal type",
            self.signal_type,
            indent = indent
        )?;
        writeln!(
            writer,
            "{:indent$}{: <22} | {}",
            "",
            "Ablation power",
            self.ablation_power,
            indent = indent
        )?;
        writeln!(
            writer,
            "{:indent$}{: <22} | {}",
            "",
            "Dual count start",
            self.dual_count_start,
            indent = indent
        )?;
        writeln!(
            writer,
            "{:indent$}{: <22} | {}",
            "",
            "Start timestamp",
            self.start_timestamp,
            indent = indent
        )?;
        writeln!(
            writer,
            "{:indent$}{: <22} | {}",
            "",
            "End timestamp",
            self.end_timestamp,
            indent = indent
        )?;
        writeln!(
            writer,
            "{:indent$}{: <22} | ({:.4} μm, {:.4} μm)",
            "",
            "ROI",
            self.roi_start_x_pos_um,
            self.roi_start_y_pos_um,
            indent = indent
        )?;
        writeln!(
            writer,
            "{:indent$}{: <22} | ({:.4} μm, {:.4} μm)",
            "",
            "",
            self.roi_end_x_pos_um,
            self.roi_end_y_pos_um,
            indent = indent
        )?;
        writeln!(
            writer,
            "{:indent$}{: <22} | {}",
            "",
            "Movement type",
            self.movement_type,
            indent = indent
        )?;
        writeln!(
            writer,
            "{:indent$}{: <22} | {:?}",
            "",
            "Segment data format",
            self.segment_data_format,
            indent = indent
        )?;
        writeln!(
            writer,
            "{:indent$}{: <22} | {}",
            "",
            "Value bytes",
            self.value_bytes,
            indent = indent
        )?;
        writeln!(
            writer,
            "{:indent$}{: <22} | {}",
            "",
            "Plume start",
            self.plume_start,
            indent = indent
        )?;
        writeln!(
            writer,
            "{:indent$}{: <22} | {}",
            "",
            "Plume end",
            self.plume_end,
            indent = indent
        )?;
        writeln!(
            writer,
            "{:indent$}{: <22} | {}",
            "",
            "Template",
            self.template,
            indent = indent
        )?;

        Ok(())
    }
}

impl<T: Seek + Read> fmt::Display for Acquisition<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.print(f, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs::File;
    use std::time::Instant;

    use crate::transform::AffineTransform;
    use nalgebra::Vector3;

    use std::io::{BufReader};

    use std::io::Cursor;
    use image::io::Reader as ImageReader;
    use image::imageops::FilterType;
    use image::ImageFormat;
    use image::GenericImageView; 

    #[test]
    fn it_works() {
        let start = Instant::now();
        let filename = "/home/alan/Documents/Work/IMC/set1.mcd";

        let duration = start.elapsed();

        //while parser.has_errors() {
        //    println!("{}", parser.pop_error_front().unwrap());
        //}

        let file = BufReader::new(File::open(filename).unwrap());
        let mcd = MCD::parse(file, filename);

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

        let slide = mcd.get_slide(&1).unwrap();

        //std::fs::write("slide.jpeg", slide.get_image().unwrap())
        //    .expect("Unable to write file");

        let mut reader = ImageReader::new(Cursor::new(mcd.get_slide(&1).unwrap().get_image().unwrap()));
        println!("Read in !");
        reader.set_format(ImageFormat::Jpeg);
        let slide_image = reader.decode().unwrap();
        //let slide_image = ImageReader::open("slide.jpeg").unwrap().decode().unwrap();


        println!("Decoded !");

        slide_image.save("slide.jpeg").unwrap();

        println!("Saved !");

        let mut resized_image = slide_image.resize_exact(7500, 2500, FilterType::Nearest).to_rgb8();
        println!("Resized !");

        for panorama in slide.get_panoramas() {
            let mut reader = ImageReader::new(Cursor::new(panorama.get_image().unwrap()));
            reader.set_format(ImageFormat::Png);
            let panorama_image = reader.decode().unwrap();
            
            let bounding_box = panorama.slide_bounding_box();

            println!("[Panorama] Bounding box = {:?}", bounding_box);
            println!("[Panorama] Transform = {:?}", panorama.to_slide_transform());

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
            }
            
            //panorama_image.resize_exact()

            //let transform = panorama.to_slide_transform();
        }

        resized_image.save("slide_resize.jpeg").unwrap();

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

        let transform = AffineTransform::from_points(points, points2);

        //println!("{}", combined_xml);
    }
}
