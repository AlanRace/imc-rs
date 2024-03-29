use core::fmt;
use std::{
    collections::{HashMap, HashSet},
    io::{BufReader, Cursor, Read, Seek, SeekFrom},
    sync::{Arc, Mutex, MutexGuard},
};

use byteorder::{LittleEndian, ReadBytesExt};
use image::ImageFormat;
use nalgebra::Vector2;

use crate::{
    channel::{AcquisitionChannel, ChannelIdentifier},
    convert::DCMLocation,
    error::{MCDError, Result},
    mcd::AcquisitionXML,
    transform::AffineTransform,
    BoundingBox, ChannelImage, OnSlide, OpticalImage, Print, Region,
};

#[derive(Debug, Clone)]
pub enum DataFormat {
    Float,
}

// #[derive(Debug)]
// pub struct DataLocation {
//     pub reader: Arc<Mutex<BufReader<File>>>,

//     pub offsets: Vec<u64>,
//     pub sizes: Vec<u64>,
// }

/// AcquisitionIdentifier is a way of identifying a specific acquisition
#[derive(Debug)]
pub enum AcquisitionIdentifier {
    /// Identified by unique identifier
    Id(u16),
    /// Identified by the number specifying the order in which the acquisition was acquired
    Order(i16),
    /// Match the description of the acquistion (specified by the user)
    Description(String),
}

impl AcquisitionIdentifier {
    /// Create an acquisition identifier based on a description
    pub fn description(description: &str) -> Self {
        AcquisitionIdentifier::Description(description.into())
    }
}

impl From<&str> for AcquisitionIdentifier {
    fn from(description: &str) -> Self {
        Self::Description(description.into())
    }
}

impl fmt::Display for AcquisitionIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AcquisitionIdentifier::Id(id) => {
                write!(f, "acquisition id: {}", id)
            }
            AcquisitionIdentifier::Order(order) => {
                write!(f, "acquisition order: {}", order)
            }
            AcquisitionIdentifier::Description(description) => {
                write!(f, "acquisition description: {}", description)
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ProfilingType {
    Global,
}

/// Trait describing a collection of acquisitions and providing methods to summarise
/// the collection (e.g. to get the list of channels)
pub trait Acquisitions {
    /// Returns a list of unique channels from the collection of acquisitions
    fn channels(&self) -> Vec<&AcquisitionChannel>;
}

impl<R> Acquisitions for Vec<&Acquisition<R>> {
    fn channels(&self) -> Vec<&AcquisitionChannel> {
        let mut channels = HashMap::new();

        for acquisition in self {
            for channel in acquisition.channels() {
                if !channels.contains_key(channel.name()) {
                    channels.insert(channel.name(), channel);
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
}

/// Acquisition represents a single region analysed by IMC.
#[derive(Debug)]
pub struct Acquisition<R> {
    pub(crate) reader: Option<Arc<Mutex<BufReader<R>>>>,
    pub(crate) dcm_location: Option<DCMLocation>,

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

    profiling_type: Option<ProfilingType>,

    channels: Vec<AcquisitionChannel>,
}

impl<R> Clone for Acquisition<R> {
    fn clone(&self) -> Self {
        Self {
            reader: self.reader.clone(),
            dcm_location: self.dcm_location.clone(),
            id: self.id,
            description: self.description.clone(),
            ablation_power: self.ablation_power,
            ablation_distance_between_shots_x: self.ablation_distance_between_shots_x,
            ablation_distance_between_shots_y: self.ablation_distance_between_shots_y,
            ablation_frequency: self.ablation_frequency,
            acquisition_roi_id: self.acquisition_roi_id,
            order_number: self.order_number,
            signal_type: self.signal_type.clone(),
            dual_count_start: self.dual_count_start.clone(),
            data_start_offset: self.data_start_offset,
            data_end_offset: self.data_end_offset,
            start_timestamp: self.start_timestamp.clone(),
            end_timestamp: self.end_timestamp.clone(),
            after_ablation_image_start_offset: self.after_ablation_image_start_offset,
            after_ablation_image_end_offset: self.after_ablation_image_end_offset,
            before_ablation_image_start_offset: self.before_ablation_image_start_offset,
            before_ablation_image_end_offset: self.before_ablation_image_end_offset,
            roi_start_x_pos_um: self.roi_start_x_pos_um,
            roi_start_y_pos_um: self.roi_start_y_pos_um,
            roi_end_x_pos_um: self.roi_end_x_pos_um,
            roi_end_y_pos_um: self.roi_end_y_pos_um,
            movement_type: self.movement_type.clone(),
            segment_data_format: self.segment_data_format.clone(),
            value_bytes: self.value_bytes,
            max_x: self.max_x,
            max_y: self.max_y,
            plume_start: self.plume_start,
            plume_end: self.plume_end,
            template: self.template.clone(),
            profiling_type: self.profiling_type,
            channels: self.channels.clone(),
        }
    }
}

pub struct SpectrumIterator<'a, R> {
    acquisition: &'a Acquisition<R>,
    reader: MutexGuard<'a, BufReader<R>>,
    buffer: Vec<u8>,
}

impl<'a, R: Seek> SpectrumIterator<'a, R> {
    fn new(acquisition: &'a Acquisition<R>) -> Self {
        let mut reader = acquisition.reader.as_ref().unwrap().lock().unwrap();

        let offset = acquisition.data_start_offset as u64;

        // TODO: Handle this properly without unwrapping
        reader.seek(SeekFrom::Start(offset)).unwrap();

        SpectrumIterator {
            acquisition,
            reader,
            buffer: vec![0u8; 4 * acquisition.channels.len()],
        }
    }
}

impl<'a, R: Read + Seek> Iterator for SpectrumIterator<'a, R> {
    type Item = Vec<f32>;

    fn next(&mut self) -> Option<Vec<f32>> {
        let cur_pos = self.reader.seek(SeekFrom::Current(0)).unwrap();
        if cur_pos >= self.acquisition.data_end_offset as u64 {
            None
        } else {
            let mut spectrum = Vec::with_capacity(self.acquisition.channels.len());

            self.reader.read_exact(&mut self.buffer).unwrap();
            let mut buffer = Cursor::new(&mut self.buffer);

            for _i in 0..self.acquisition.channels.len() {
                let float = buffer.read_f32::<LittleEndian>().unwrap();

                spectrum.push(float);
            }

            Some(spectrum)
        }
    }
}

impl<R> Acquisition<R> {
    /// Returns the ID associated with the acquisition
    pub fn id(&self) -> u16 {
        self.id
    }

    /// Returns a description of the acquisition
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Returns a number representing the order in which the acquisition was acquired (0 being first).
    pub fn order_number(&self) -> i16 {
        self.order_number
    }

    /// Returns the width of the acquired region (in pixels)
    pub fn width(&self) -> i32 {
        self.max_x
    }

    /// Returns the height of the acquired region (in pixels)
    pub fn height(&self) -> i32 {
        self.max_y
    }

    /// Returns the ablation frequency
    pub fn ablation_frequency(&self) -> f64 {
        self.ablation_frequency
    }

    /// Returns the region ID for the acquisition
    pub fn acquisition_roi_id(&self) -> i16 {
        self.acquisition_roi_id
    }

    /// Returns the profiling type for the acquisition, if one is present. This is not present in version 1 of the schema
    pub fn profiling_type(&self) -> Option<&ProfilingType> {
        self.profiling_type.as_ref()
    }

    /*fn image_data(&self, start: i64, end: i64) -> Result<Vec<u8>, std::io::Error> {
        let mutex = self
            .reader
            .as_ref()
            .expect("Should have copied the reader across");
        let reader = mutex.lock().unwrap();

        read_image_data(reader, start, end)
    }

    fn dynamic_image(&self, start: i64, end: i64) -> DynamicImage {
        let mut reader = ImageReader::new(Cursor::new(self.image_data(start, end).unwrap()));
        reader.set_format(ImageFormat::Png);
        reader.decode().unwrap()
    }*/

    /// Returns the optical image of the acquisition region prior to ablation
    pub fn before_ablation_image(&self) -> OpticalImage<R> {
        OpticalImage {
            reader: self.reader.as_ref().unwrap().clone(),
            start_offset: self.before_ablation_image_start_offset,
            end_offset: self.before_ablation_image_end_offset,
            image_format: ImageFormat::Png,
        }
        // match self.dynamic_image(
        //     self.before_ablation_image_start_offset,
        //     self.before_ablation_image_end_offset,
        // ) {
        //     DynamicImage::ImageRgba8(rgba8) => rgba8,
        //     _ => panic!("Unexpected DynamicImage type"),
        // }
    }

    /// Returns the optical image of the acquisition region after ablation
    pub fn after_ablation_image(&self) -> OpticalImage<R> {
        OpticalImage {
            reader: self.reader.as_ref().unwrap().clone(),
            start_offset: self.after_ablation_image_start_offset,
            end_offset: self.after_ablation_image_end_offset,
            image_format: ImageFormat::Png,
        }
    }

    /// Returns a list of all channels acquired within this acquisition
    pub fn channels(&self) -> &[AcquisitionChannel] {
        &self.channels
    }

    pub(crate) fn channels_mut(&mut self) -> &mut Vec<AcquisitionChannel> {
        &mut self.channels
    }

    /// Returns whether the acquisition has run to completion (checks the size of the recorded data
    /// compared to the expected data size)
    pub fn is_complete(&self) -> bool {
        let expected_size: usize = self.channels().len()
            * self.max_x as usize
            * self.max_y as usize
            * self.value_bytes as usize;
        let measured_size: usize = self.data_end_offset as usize - self.data_start_offset as usize;

        // println!("Expected: {} | Measured: {}", expected_size, measured_size);

        expected_size == measured_size
    }

    /// Returns a `Region` describing the pixel region contained within the specified bounding box
    pub fn pixels_in(&self, region: &BoundingBox<f64>) -> Option<Region> {
        let transform = self.to_slide_transform();

        let top_left = transform.transform_from_slide(region.min_x, region.min_y)?;
        let top_right = transform.transform_from_slide(region.max_x(), region.min_y)?;

        let bottom_right = transform.transform_from_slide(region.max_x(), region.max_y())?;
        let bottom_left = transform.transform_from_slide(region.min_x, region.max_y())?;

        let min_x = top_left
            .x
            .min(top_right.x)
            .min(bottom_right.x)
            .min(bottom_left.x)
            .max(0.0)
            .floor();

        let max_x = top_left
            .x
            .max(top_right.x)
            .max(bottom_right.x)
            .max(bottom_left.x)
            .min(self.width() as f64)
            .ceil();

        let min_y = top_left
            .y
            .min(top_right.y)
            .min(bottom_right.y)
            .min(bottom_left.y)
            .max(0.0)
            .floor();

        let max_y = top_left
            .y
            .max(top_right.y)
            .max(bottom_right.y)
            .max(bottom_left.y)
            .min(self.height() as f64)
            .ceil();

        Some(Region {
            x: min_x as u32,
            y: (self.height() as u32 - max_y as u32),
            width: (max_x - min_x) as u32,
            height: (max_y - min_y) as u32,
        })
    }

    /// Tests whether the acquisition is (at least partially) contained within the specified bounding box (slide coordinates).
    pub fn in_region(&self, region: &BoundingBox<f64>) -> bool {
        let slide_box = self.slide_bounding_box();

        if slide_box.min_x < region.max_x()
            && slide_box.max_x() > region.min_x
            && slide_box.max_y() > region.min_y
            && slide_box.min_y < region.max_y()
        {
            return true;
        }

        false
    }

    /// Returns the size of a single spectrum in bytes
    #[inline]
    pub fn spectrum_size(&self) -> usize {
        self.channels().len() * self.value_bytes as usize
    }

    /// Returns the number of spectra acquired as part of the acquisition
    #[inline]
    pub fn num_spectra(&self) -> usize {
        let measured_size: usize = self.data_end_offset as usize - self.data_start_offset as usize;

        measured_size / self.spectrum_size()
    }
}

impl<R: Read + Seek> Acquisition<R> {
    /// Returns the ChannelImage for the channel matching the `ChannelIdentifier`. This contains the intensities of the channel
    /// for each detected pixel, the number of valid pixels and the width and height of the image.
    pub fn channel_image<C: Into<ChannelIdentifier>>(
        &self,
        identifier: C,
        region: Option<Region>,
    ) -> Result<ChannelImage> {
        Ok(self
            .channel_images(&[identifier.into()], region)?
            .drain(..)
            .last()
            .expect("A channel image should always be returned, as we always pass one identifier"))
    }

    /// Returns array of ChannelImages for the channels matching the `ChannelIdentifier`s. This contains the intensities of the channel
    /// for each detected pixel, the number of valid pixels and the width and height of the image.
    pub fn channel_images<C: AsRef<ChannelIdentifier>>(
        &self,
        identifiers: &[C],
        region: Option<Region>,
    ) -> Result<Vec<ChannelImage>> {
        // println!("Searching identifiers: {:?}", identifiers);
        // println!("Searching from channels: {:?}", self.channels());

        let channels: Vec<_> = identifiers
            .iter()
            .map(|identifier| {
                self.channel(identifier).ok_or(MCDError::InvalidChannel {
                    channel: identifier.as_ref().clone(),
                })
            })
            .collect::<Result<Vec<_>>>()?;

        let order_numbers: Vec<_> = channels
            .iter()
            .map(|channel| channel.order_number() as usize)
            .collect();

        let region = match region {
            Some(region) => region,
            None => crate::Region {
                x: 0,
                y: 0,
                width: self.width() as u32,
                height: self.height() as u32,
            },
        };

        let last_row = self.num_spectra() / self.width() as usize;
        let last_col = self.width() as usize - (self.num_spectra() % self.width() as usize);

        let valid_region_row = (region.y + region.height).min(last_row as u32);
        let valid_region_col = (region.x + region.width).min(last_col as u32);

        // println!("{} / {}", valid_region_row, region.y);
        let valid_pixels = if region.y >= valid_region_row {
            (valid_region_row - 1) * region.width + valid_region_col
        } else {
            ((valid_region_row - region.y - 1) * region.width) + (valid_region_col - region.x)
        };

        let mut data = if let Some(data_location) = &self.dcm_location {
            data_location.read_channels(&order_numbers, &region)?
        } else {
            let mut data: Vec<Vec<f32>> =
                vec![
                    Vec::with_capacity((region.width * region.height) as usize);
                    identifiers.len()
                ];

            let order_hash: HashSet<usize> = HashSet::from_iter(order_numbers.iter().copied());

            for y in region.y..(region.y + region.height) {
                for x in region.x..(region.x + region.width) {
                    for (channel_index, intensity) in self
                        .spectrum(x, y)?
                        .iter()
                        .enumerate()
                        .filter(|(index, _intensity)| order_hash.contains(index))
                        .map(|(_, intensity)| *intensity)
                        .enumerate()
                    {
                        data[channel_index].push(intensity);
                    }
                }
            }

            data
        };

        let images: Vec<_> = data
            .drain(..)
            .zip(channels.iter())
            .map(|(data, channel)| {
                let mut min_value = f32::MAX;
                let mut max_value = f32::MIN;

                for &data_point in data.iter() {
                    if data_point < min_value {
                        min_value = data_point;
                    }
                    if data_point > max_value {
                        max_value = data_point;
                    }
                }

                ChannelImage {
                    region,
                    acquisition_id: channel.acquisition_id(),
                    name: channel.name().to_string(),
                    label: channel.label().to_string(),
                    range: (min_value, max_value),
                    valid_pixels: valid_pixels as usize,
                    data,
                }
            })
            .collect();

        Ok(images)

        // Ok(ChannelImage {
        //     region,
        //     range: (min_value, max_value),
        //     valid_pixels: valid_pixels as usize,
        //     data,
        // })
    }

    /// Returns the channel which matches the given identifier, or None if no match found
    pub fn channel<C: AsRef<ChannelIdentifier>>(
        &self,
        identifier: C,
    ) -> Option<&AcquisitionChannel> {
        self.channels
            .iter()
            .find(|&channel| channel.is(identifier.as_ref()))
    }

    // pub fn channel_index(&self, identifier: &ChannelIdentifier) -> Option<usize> {
    //     for (index, channel) in self.channels.iter().enumerate() {
    //         if channel.is(identifier) {
    //             return Some(index);
    //         }
    //     }

    //     None
    // }

    // There are a number of potential issues with the ROI positions that we attempt to fix here
    pub(crate) fn fix_roi_positions(&mut self) {
        // In version 2 of the schema, it seems like ROIStartXPosUm and ROIStartYPosUm are 1000x what they should be, so try and detect this and correct for it
        if self.roi_start_x_pos_um > 75000.0 {
            self.roi_start_x_pos_um /= 1000.0;
        }

        // In version 2 of the schema, it seems like ROIStartXPosUm and ROIStartYPosUm are 1000x what they should be, so try and detect this and correct for it
        if self.roi_start_y_pos_um > 75000.0 {
            self.roi_start_y_pos_um /= 1000.0;
        }

        // There seems to be a bug where the start and end x pos is recorded as the same value
        if (self.roi_start_x_pos_um == self.roi_end_x_pos_um) || self.roi_end_x_pos_um == 0.0 {
            self.roi_end_x_pos_um = self.roi_start_x_pos_um
                + (self.max_x as f64 * self.ablation_distance_between_shots_x);
        }

        if self.roi_end_y_pos_um == 0.0 {
            self.roi_end_y_pos_um = self.roi_start_y_pos_um
                - (self.max_y as f64 * self.ablation_distance_between_shots_y);
        }
    }
}

impl<R: Read + Seek> Acquisition<R> {
    /// Provides an iterator over all spectra (each pixel) within the acquisition
    pub fn spectra(&self) -> SpectrumIterator<R> {
        SpectrumIterator::new(self)
    }

    /// Returns a spectrum at the specified (x, y) coordinate
    pub fn spectrum(&self, x: u32, y: u32) -> Result<Vec<f32>> {
        let index = y as usize * self.max_x as usize + x as usize;

        if index >= self.num_spectra() {
            return Err(MCDError::InvalidIndex {
                index,
                num_spectra: self.num_spectra(),
            });
        }

        let offset = self.data_start_offset as u64
            + (index * self.channels.len() * self.value_bytes as usize) as u64;

        let mut spectrum = Vec::with_capacity(self.channels.len());
        let mut reader = self.reader.as_ref().unwrap().lock().unwrap();

        // TODO: Handle this properly without unwrapping
        reader.seek(SeekFrom::Start(offset))?;

        let mut buffer = [0u8; 4];
        for _i in 0..self.channels.len() {
            reader.read_exact(&mut buffer)?;
            let float = f32::from_le_bytes(buffer);
            spectrum.push(float);
        }

        Ok(spectrum)
    }
}

impl<R> OnSlide for Acquisition<R> {
    /// Returns the affine transformation from pixel coordinates within the acquisition to to the slide coordinates (μm)
    fn to_slide_transform(&self) -> AffineTransform<f64> {
        let mut moving_points = Vec::new();
        let mut fixed_points = Vec::new();

        moving_points.push(Vector2::new(
            self.roi_start_x_pos_um,
            self.roi_start_y_pos_um,
            //25000.0 - self.roi_start_y_pos_um,
        ));
        moving_points.push(Vector2::new(
            self.roi_end_x_pos_um,
            self.roi_start_y_pos_um,
            //25000.0 - self.roi_start_y_pos_um,
        ));
        moving_points.push(Vector2::new(
            self.roi_start_x_pos_um,
            self.roi_end_y_pos_um,
            //25000.0 - self.roi_end_y_pos_um,
        ));
        //moving_points.push(Vector2::new(roi_end_x_pos_um, self.roi_end_y_pos_um));

        let y = (self.roi_start_y_pos_um - self.roi_end_y_pos_um)
            / self.ablation_distance_between_shots_y;

        fixed_points.push(Vector2::new(0.0, y));
        fixed_points.push(Vector2::new(self.max_x as f64, y));
        //fixed_points.push(Vector2::new(0.0, self.max_y as f64));
        fixed_points.push(Vector2::new(0.0, 0.0));

        //fixed_points.push(Vector2::new(self.max_x as f64, self.max_y as f64));

        AffineTransform::from_points(moving_points, fixed_points)
    }

    /// Returns the bounding box encompasing the acquisition area on the slide (in μm)
    fn slide_bounding_box(&self) -> BoundingBox<f64> {
        BoundingBox {
            min_x: f64::min(self.roi_start_x_pos_um, self.roi_end_x_pos_um),
            min_y: f64::min(self.roi_start_y_pos_um, self.roi_end_y_pos_um),
            width: (self.roi_end_x_pos_um - self.roi_start_x_pos_um).abs(),
            height: (self.roi_end_y_pos_um - self.roi_start_y_pos_um).abs(),
        }
    }
}

#[rustfmt::skip]
impl<R> Print for Acquisition<R> {
    fn print<W: fmt::Write + ?Sized>(&self, writer: &mut W, indent: usize) -> fmt::Result {
        write!(writer, "{:indent$}", "", indent = indent)?;
        writeln!(writer, "{:-^1$}", "Acquisition", 48)?;

        writeln!(writer, "{:indent$}{: <22} | {}", "", "ID",          self.id,          indent = indent)?;
        writeln!(writer, "{:indent$}{: <22} | {}", "", "Description", self.description, indent = indent)?;
        writeln!(writer, "{:indent$}{: <22} | {}", "", "Order number", self.order_number, indent = indent)?;
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

impl<R> fmt::Display for Acquisition<R> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.print(f, 0)
    }
}

impl<R> From<AcquisitionXML> for Acquisition<R> {
    fn from(acquisition: AcquisitionXML) -> Self {
        Acquisition {
            reader: None,
            dcm_location: None,

            id: acquisition.id.unwrap(),
            description: acquisition.description.unwrap(),
            ablation_power: acquisition.ablation_power.unwrap(),
            ablation_distance_between_shots_x: acquisition
                .ablation_distance_between_shots_x
                .unwrap(),
            ablation_distance_between_shots_y: acquisition
                .ablation_distance_between_shots_y
                .unwrap(),
            ablation_frequency: acquisition.ablation_frequency.unwrap(),
            acquisition_roi_id: acquisition.acquisition_roi_id.unwrap(),
            order_number: acquisition.order_number.unwrap(),
            signal_type: acquisition.signal_type.unwrap(),
            dual_count_start: acquisition.dual_count_start.unwrap(),
            data_start_offset: acquisition.data_start_offset.unwrap(),
            data_end_offset: acquisition.data_end_offset.unwrap(),
            start_timestamp: acquisition.start_timestamp.unwrap(),
            end_timestamp: acquisition.end_timestamp.unwrap(),
            after_ablation_image_start_offset: acquisition
                .after_ablation_image_start_offset
                .unwrap(),
            after_ablation_image_end_offset: acquisition.after_ablation_image_end_offset.unwrap(),
            before_ablation_image_start_offset: acquisition
                .before_ablation_image_start_offset
                .unwrap(),
            before_ablation_image_end_offset: acquisition.before_ablation_image_end_offset.unwrap(),
            roi_start_x_pos_um: acquisition.roi_start_x_pos_um.unwrap(),
            roi_start_y_pos_um: acquisition.roi_start_y_pos_um.unwrap(),
            roi_end_x_pos_um: acquisition.roi_end_x_pos_um.unwrap(),
            roi_end_y_pos_um: acquisition.roi_end_y_pos_um.unwrap(),
            movement_type: acquisition.movement_type.unwrap(),
            segment_data_format: acquisition.segment_data_format.unwrap(),
            value_bytes: acquisition.value_bytes.unwrap(),
            max_x: acquisition.max_x.unwrap(),
            max_y: acquisition.max_y.unwrap(),
            plume_start: acquisition.plume_start.unwrap(),
            plume_end: acquisition.plume_end.unwrap(),
            template: acquisition.template.unwrap(),

            profiling_type: acquisition.profiling_type,

            channels: Vec::new(),
        }
    }
}
