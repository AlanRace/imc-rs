mod parser;

use std::collections::HashMap;
use std::io::prelude::*;
use std::io::SeekFrom;
use std::sync::Arc;

use std::convert::TryInto;

const BUF_SIZE: usize = 4096;

fn find_mcd_start(chunk: &std::vec::Vec<u8>, chunk_size: usize) -> usize {
    for start_index in 0..chunk_size {
        match std::str::from_utf8(&chunk[start_index..]) {
            Ok(data) => {
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

fn get_image_data(
    mut source: impl Read + Seek,
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

pub struct MCD {
    xmlns: String,

    slide: Slide,
    panoramas: HashMap<String, Panorama>,
    //acquisition_order: Vec<String>,
    acquisitions: HashMap<String, Arc<Acquisition>>,
    //acquisition_rois: Vec<AcquisitionROI>,
    roi_points: Vec<ROIPoint>,
}

impl MCD {
    pub fn new(xmlns: &str) -> MCD {
        MCD {
            xmlns: xmlns.into(),
            slide: Slide::new(),
            panoramas: HashMap::new(),
            //acquisition_channels: Vec::new(),
            //acquisition_order: Vec::new(),
            acquisitions: HashMap::new(),
            //acquisition_rois: Vec::new(),
            roi_points: Vec::new(),
        }
    }


    /*pub(crate) fn add_acquisition_channel(&mut self, channel: AcquisitionChannel) {
        let acquisition_id = channel.acquisition_id.expect("Must have an AcquisitionID or won't know which Acquisition the AcquisitionChannel belongs to");
println!("{:?}", self.acquisitions);
        

        panic!("No acquistion with ID: {:?}", acquisition_id);
    }*/
}

pub struct MCDPublic {}

#[derive(Debug)]
pub struct Slide {
    id: Option<String>,
    uid: Option<String>,
    description: Option<String>,
    filename: Option<String>,
    slide_type: Option<String>,
    width_um: Option<f64>,
    height_um: Option<f64>,

    image_start_offset: Option<i64>,
    image_end_offset: Option<i64>,
    image_file: Option<String>,

    sw_version: Option<String>,
}

impl Slide {
    pub fn new() -> Slide {
        Slide {
            id: None,
            uid: None,
            description: None,
            filename: None,
            slide_type: None,
            width_um: None,
            height_um: None,
            image_start_offset: None,
            image_end_offset: None,
            image_file: None,
            sw_version: None,
        }
    }
}

#[derive(Debug)]
pub enum ImageFormat {
    Png,
}

#[derive(Debug)]
pub struct Panorama {
    id: Option<String>,
    slide_id: Option<String>,
    description: Option<String>,
    slide_x1_pos_um: Option<f64>,
    slide_y1_pos_um: Option<f64>,
    slide_x2_pos_um: Option<f64>,
    slide_y2_pos_um: Option<f64>,
    slide_x3_pos_um: Option<f64>,
    slide_y3_pos_um: Option<f64>,
    slide_x4_pos_um: Option<f64>,
    slide_y4_pos_um: Option<f64>,

    image_start_offset: Option<i64>,
    image_end_offset: Option<i64>,
    pixel_width: Option<i64>,
    pixel_height: Option<i64>,
    image_format: Option<ImageFormat>,
    pixel_scale_coef: Option<f64>,

    acquisitions: HashMap<String, Arc<Acquisition>>,
}

impl Panorama {
    pub fn new() -> Panorama {
        Panorama {
            id: None,
            slide_id: None,
            description: None,
            slide_x1_pos_um: None,
            slide_y1_pos_um: None,
            slide_x2_pos_um: None,
            slide_y2_pos_um: None,
            slide_x3_pos_um: None,
            slide_y3_pos_um: None,
            slide_x4_pos_um: None,
            slide_y4_pos_um: None,
            image_start_offset: None,
            image_end_offset: None,
            pixel_width: None,
            pixel_height: None,
            image_format: None,
            pixel_scale_coef: None,

            acquisitions: HashMap::new(),
        }
    }

    pub fn get_image(&self, source: impl Read + Seek) -> Result<Vec<u8>, std::io::Error> {
        get_image_data(
            source,
            self.image_start_offset.unwrap(),
            self.image_end_offset.unwrap(),
        )
    }
}

#[derive(Debug)]
pub struct AcquisitionChannel {
    id: String,
    channel_name: String,
    order_number: i16,
    acquisition_id: String,
    channel_label: String,
}



#[derive(Debug)]
enum DataFormat {
    Float,
}

#[derive(Debug)]
pub struct Acquisition {
    id: String,
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

impl Acquisition {

    pub fn get_before_ablation_image(
        &self,
        source: impl Read + Seek,
    ) -> Result<Vec<u8>, std::io::Error> {
        get_image_data(
            source,
            self.before_ablation_image_start_offset,
            self.before_ablation_image_end_offset,
        )
    }

    pub fn get_after_ablation_image(
        &self,
        source: impl Read + Seek,
    ) -> Result<Vec<u8>, std::io::Error> {
        get_image_data(
            source,
            self.after_ablation_image_start_offset,
            self.after_ablation_image_end_offset,
        )
    }
}

#[derive(Debug)]
enum ROIType {
    Acquisition,
}

#[derive(Debug)]
pub struct AcquisitionROI {
    id: Option<String>,
    panorama_id: Option<String>,
    roi_type: Option<ROIType>,
}

impl AcquisitionROI {
    pub fn new() -> AcquisitionROI {
        AcquisitionROI {
            id: None,
            panorama_id: None,
            roi_type: None,
        }
    }
}

#[derive(Debug)]
pub struct ROIPoint {
    id: Option<String>,
    acquisition_roi_id: Option<String>,
    order_number: Option<i16>,
    slide_x_pos_um: Option<f64>,
    slide_y_pos_um: Option<f64>,
    panorama_pixel_x_pos: Option<i32>,
    panorama_pixel_y_pos: Option<i32>,
}

impl ROIPoint {
    pub fn new() -> ROIPoint {
        ROIPoint {
            id: None,
            acquisition_roi_id: None,
            order_number: None,
            slide_x_pos_um: None,
            slide_y_pos_um: None,
            panorama_pixel_x_pos: None,
            panorama_pixel_y_pos: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::parser::MCDParser;
    use crate::parser::ParserState;

    use super::*;

    use quick_xml::events::{attributes::Attribute, Event};
    use std::fs::File;
    use std::io::prelude::*;
    use std::io::SeekFrom;
    use std::time::Instant;

    use std::convert::TryInto;

    #[test]
    fn it_works() {
        let mut parser = MCDParser::new();
        let start = Instant::now();
        let filename = "/home/alan/Documents/Work/IMC/set1.mcd";

        let mut file = File::open(filename).unwrap();

        let chunk_size: i64 = 1000;
        let mut cur_offset: i64 = 0;

        //let mut strings = Vec::<String>::new();

        let mut buf_u8 = vec![0; chunk_size.try_into().unwrap()];

        loop {
            file.seek(SeekFrom::End(-cur_offset - chunk_size)).unwrap();

            //let mut buf = String::new();
            file.read_exact(&mut buf_u8).unwrap();
            // .read_to_string(&mut buf).unwrap();

            match std::str::from_utf8(&buf_u8) {
                Ok(_data) => {} //strings.push(data.to_owned()),
                Err(_error) => {
                    // Found the final chunk, so find the start point
                    let start_index = find_mcd_start(&buf_u8, chunk_size.try_into().unwrap());

                    let total_size = cur_offset + chunk_size - (start_index as i64);
                    buf_u8 = vec![0; total_size.try_into().unwrap()];

                    file.seek(SeekFrom::End(-total_size)).unwrap();
                    file.read_exact(&mut buf_u8).unwrap();

                    println!("Start Index: {}", start_index);

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

        let duration = start.elapsed();

        //while parser.has_errors() {
        //    println!("{}", parser.pop_error_front().unwrap());
        //}

        let mcd = parser.get_mcd();

        //println!("{:?}", mcd.slide);
        //println!("{:?}", mcd.panoramas);
        //for (id, acquisition) in &mcd.acquisitions {
        //    println!("{:?}: {:?}", id, acquisition);
        //}
        
        //println!("{:?}", mcd.acquisitions[0]);
        println!("{:?}", mcd.roi_points[0]);
        std::fs::write("tmp.xml", combined_xml).expect("Unable to write file");

        std::fs::write("tmp.png", mcd.panoramas.get("8").unwrap().get_image(&mut file).unwrap())
            .expect("Unable to write file");

        println!("Time elapsed when parsing is: {:?}", duration);

        //println!("{}", combined_xml);
    }
}
