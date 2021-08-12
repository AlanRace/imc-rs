use std::fs::File;
use std::io::prelude::*;
use std::io::SeekFrom;

use quick_xml::events::{attributes::Attribute, Event};
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

pub struct MCD {
    xmlns: String,

    slide: Slide,
    panoramas: Vec<Panorama>,
    acquisition_channels: Vec<AcquisitionChannel>,
    acquisitions: Vec<Acquisition>,
    acquisition_rois: Vec<AcquisitionROI>,
    roi_points: Vec<ROIPoint>,
}

impl MCD {
    pub fn new(xmlns: &str) -> MCD {
        MCD {
            xmlns: xmlns.into(),
            slide: Slide::new(),
            panoramas: Vec::new(),
            acquisition_channels: Vec::new(),
            acquisitions: Vec::new(),
            acquisition_rois: Vec::new(),
            roi_points: Vec::new(),
        }
    }
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
        }
    }

    pub fn get_image(&self, file: &mut File) -> Result<Vec::<u8>, std::io::Error> {
        let mut image_start_offset = self.image_start_offset.unwrap();
        
        // Add an offset to skip the C# Drawing data
        image_start_offset += 161;
        let image_size = self.image_end_offset.unwrap() - image_start_offset;

        let mut buf_u8 = vec![0; image_size.try_into().unwrap()];

        match file.seek(SeekFrom::Start(image_start_offset as u64)) {
            Ok(_seek) => {
                match file.read_exact(&mut buf_u8) {
                    Ok(()) => Ok(buf_u8),
                    Err(error) => Err(error)
                } 
            }
            Err(error) => Err(error)
        }
    }
}



#[derive(Debug)]
pub struct AcquisitionChannel {
    id: Option<String>,
    channel_name: Option<String>,
    order_number: Option<i16>,
    acquisition_id: Option<String>,
    channel_label: Option<String>,
}

impl AcquisitionChannel {
    pub fn new() -> AcquisitionChannel {
        AcquisitionChannel {
            id: None,
            channel_name: None,
            order_number: None,
            acquisition_id: None,
            channel_label: None,
        }
    }
}

#[derive(Debug)]
enum DataFormat {
    Float,
}

#[derive(Debug)]
pub struct Acquisition {
    id: Option<String>,
    description: Option<String>,
    ablation_power: Option<f64>,
    ablation_distance_between_shots_x: Option<f64>,
    ablation_distance_between_shots_y: Option<f64>,
    ablation_frequency: Option<f64>,
    acquisition_roi_id: Option<i16>,
    order_number: Option<i16>,
    signal_type: Option<String>,
    dual_count_start: Option<String>,
    data_start_offset: Option<i64>,
    data_end_offset: Option<i64>,
    start_timestamp: Option<String>,
    end_timestamp: Option<String>,
    after_ablation_image_start_offset: Option<i64>,
    after_ablation_image_end_offset: Option<i64>,
    before_ablation_image_start_offset: Option<i64>,
    before_ablation_image_end_offset: Option<i64>,
    roi_start_x_pos_um: Option<f64>,
    roi_start_y_pos_um: Option<f64>,
    roi_end_x_pos_um: Option<f64>,
    roi_end_y_pos_um: Option<f64>,
    movement_type: Option<String>,
    segment_data_format: Option<DataFormat>,
    value_bytes: Option<u8>,
    max_x: Option<i32>,
    max_y: Option<i32>,
    plume_start: Option<i32>,
    plume_end: Option<i32>,
    template: Option<String>,
}

impl Acquisition {
    pub fn new() -> Acquisition {
        Acquisition {
            id: None,
            description: None,
            ablation_power: None,
            ablation_distance_between_shots_x: None,
            ablation_distance_between_shots_y: None,
            ablation_frequency: None,
            acquisition_roi_id: None,
            order_number: None,
            signal_type: None,
            dual_count_start: None,
            data_start_offset: None,
            data_end_offset: None,
            start_timestamp: None,
            end_timestamp: None,
            after_ablation_image_start_offset: None,
            after_ablation_image_end_offset: None,
            before_ablation_image_start_offset: None,
            before_ablation_image_end_offset: None,
            roi_start_x_pos_um: None,
            roi_start_y_pos_um: None,
            roi_end_x_pos_um: None,
            roi_end_y_pos_um: None,
            movement_type: None,
            segment_data_format: None,
            value_bytes: None,
            max_x: None,
            max_y: None,
            plume_start: None,
            plume_end: None,
            template: None,
        }
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

#[derive(Clone, Copy)]
pub enum ParserState {
    Start,
    ProcessingSlide,
    ProcessingID,
    ProcessingUID,
    ProcessingDescription,
    ProcessingFilename,
    ProcessingSlideType,
    ProcessingWidthUm,
    ProcessingHeightUm,
    ProcessingImageStartOffset,
    ProcessingImageEndOffset,
    ProcessingImageFile,
    ProcessingSwVersion,
    ProcessingSlideID,
    ProcessingSlideX1PosUm,
    ProcessingSlideY1PosUm,
    ProcessingSlideX2PosUm,
    ProcessingSlideY2PosUm,
    ProcessingSlideX3PosUm,
    ProcessingSlideY3PosUm,
    ProcessingSlideX4PosUm,
    ProcessingSlideY4PosUm,
    ProcessingPixelWidth,
    ProcessingPixelHeight,
    ProcessingImageFormat,
    ProcessingPixelScaleCoef,
    ProcessingAcquisition,
    ProcessingAblationPower,
    ProcessingAblationDistanceBetweenShotsX,
    ProcessingAblationDistanceBetweenShotsY,
    ProcessingAblationFrequency,
    ProcessingAcquisitionROIID,
    ProcessingSignalType,
    ProcessingDualCountStart,
    ProcessingDataStartOffset,
    ProcessingDataEndOffset,
    ProcessingStartTimeStamp,
    ProcessingEndTimeStamp,
    ProcessingAfterAblationImageEndOffset,
    ProcessingAfterAblationImageStartOffset,
    ProcessingBeforeAblationImageEndOffset,
    ProcessingBeforeAblationImageStartOffset,
    ProcessingROIStartXPosUm,
    ProcessingROIStartYPosUm,
    ProcessingROIEndXPosUm,
    ProcessingROIEndYPosUm,
    ProcessingMovementType,
    ProcessingSegmentDataFormat,
    ProcessingValueBytes,
    ProcessingMaxY,
    ProcessingMaxX,
    ProcessingPlumeStart,
    ProcessingPlumeEnd,
    ProcessingTemplate,
    ProcessingAcquisitionChannel,
    ProcessingChannelName,
    ProcessingOrderNumber,
    ProcessingAcquisitionID,
    ProcessingChannelLabel,
    ProcessingAcquisitionROI,
    ProcessingPanoramaID,
    ProcessingROIType,
    ProcessingROIPoint,
    ProcessingSlideXPosUm,
    ProcessingSlideYPosUm,
    ProcessingPanoramaPixelXPos,
    ProcessingPanoramaPixelYPos,
    ProcessingPanorama,
    Processing,
    Error,
    FatalError, // Must stop here
    Finished,
}

pub struct MCDParser {
    pub(crate) state: ParserState,
    pub(crate) sub_state: ParserState,
    //pub(super) history: Vec<String>,
    pub(crate) errors: std::collections::VecDeque<String>,

    pub(crate) current_mcd: Option<MCD>,

    pub(crate) current_panorama: Option<Panorama>,
    pub(crate) current_acquisition_channel: Option<AcquisitionChannel>,
    pub(crate) current_acquisition: Option<Acquisition>,
    pub(crate) current_acquisition_roi: Option<AcquisitionROI>,
    pub(crate) current_roi_point: Option<ROIPoint>,
}

impl MCDParser {
    pub fn new() -> MCDParser {
        MCDParser {
            state: ParserState::Start,
            sub_state: ParserState::Start,
            errors: std::collections::VecDeque::new(),

            current_mcd: None,
            current_panorama: None,
            current_acquisition_channel: None,
            current_acquisition: None,
            current_acquisition_roi: None,
            current_roi_point: None,
        }
    }

    pub fn current_state(&self) -> ParserState {
        self.state
    }

    pub fn has_errors(&self) -> bool {
        self.errors.len() > 0
    }

    pub fn pop_error_front(&mut self) -> Option<String> {
        self.errors.pop_front()
    }

    pub fn pop_error_back(&mut self) -> Option<String> {
        self.errors.pop_back()
    }

    pub fn process(&mut self, ev: Event) {
        match ev {
            Event::Start(e) | Event::Empty(e) => match e.local_name() {
                b"MCDSchema" => {
                    // TODO: get xmlns
                    self.current_mcd = Some(MCD::new(""))
                }
                b"Slide" => {
                    //Wself.current_mcd.unwrap().slide = Some(Slide::new());
                    self.state = ParserState::ProcessingSlide
                }
                b"Panorama" => {
                    self.current_panorama = Some(Panorama::new());
                    self.state = ParserState::ProcessingPanorama
                }
                b"AcquisitionROI" => {
                    self.current_acquisition_roi = Some(AcquisitionROI::new());
                    self.state = ParserState::ProcessingAcquisitionROI
                }
                b"ROIPoint" => {
                    self.current_roi_point = Some(ROIPoint::new());
                    self.state = ParserState::ProcessingROIPoint
                }
                b"AcquisitionChannel" => {
                    self.current_acquisition_channel = Some(AcquisitionChannel::new());
                    self.state = ParserState::ProcessingAcquisitionChannel
                }
                b"Acquisition" => {
                    self.current_acquisition = Some(Acquisition::new());
                    self.state = ParserState::ProcessingAcquisition
                }
                b"ID" => self.sub_state = ParserState::ProcessingID,
                b"UID" => self.sub_state = ParserState::ProcessingUID,
                b"Description" => self.sub_state = ParserState::ProcessingDescription,
                b"Filename" => self.sub_state = ParserState::ProcessingFilename,
                b"SlideType" => self.sub_state = ParserState::ProcessingSlideType,
                b"WidthUm" => self.sub_state = ParserState::ProcessingWidthUm,
                b"HeightUm" => self.sub_state = ParserState::ProcessingHeightUm,
                b"ImageStartOffset" => self.sub_state = ParserState::ProcessingImageStartOffset,
                b"ImageEndOffset" => self.sub_state = ParserState::ProcessingImageEndOffset,
                b"ImageFile" => self.sub_state = ParserState::ProcessingImageFile,
                b"SwVersion" => self.sub_state = ParserState::ProcessingSwVersion,
                b"SlideID" => self.sub_state = ParserState::ProcessingSlideID,
                b"SlideX1PosUm" => self.sub_state = ParserState::ProcessingSlideX1PosUm,
                b"SlideY1PosUm" => self.sub_state = ParserState::ProcessingSlideY1PosUm,
                b"SlideX2PosUm" => self.sub_state = ParserState::ProcessingSlideX2PosUm,
                b"SlideY2PosUm" => self.sub_state = ParserState::ProcessingSlideY2PosUm,
                b"SlideX3PosUm" => self.sub_state = ParserState::ProcessingSlideX3PosUm,
                b"SlideY3PosUm" => self.sub_state = ParserState::ProcessingSlideY3PosUm,
                b"SlideX4PosUm" => self.sub_state = ParserState::ProcessingSlideX4PosUm,
                b"SlideY4PosUm" => self.sub_state = ParserState::ProcessingSlideY4PosUm,
                b"PixelWidth" => self.sub_state = ParserState::ProcessingPixelWidth,
                b"PixelHeight" => self.sub_state = ParserState::ProcessingPixelHeight,
                b"ImageFormat" => self.sub_state = ParserState::ProcessingImageFormat,
                b"PixelScaleCoef" => self.sub_state = ParserState::ProcessingPixelScaleCoef,
                b"ChannelName" => self.sub_state = ParserState::ProcessingChannelName,
                b"OrderNumber" => self.sub_state = ParserState::ProcessingOrderNumber,
                b"AcquisitionID" => self.sub_state = ParserState::ProcessingAcquisitionID,
                b"ChannelLabel" => self.sub_state = ParserState::ProcessingChannelLabel,
                b"AblationPower" => self.sub_state = ParserState::ProcessingAblationPower,
                b"AblationDistanceBetweenShotsX" => self.sub_state = ParserState::ProcessingAblationDistanceBetweenShotsX,
                b"AblationDistanceBetweenShotsY" => self.sub_state = ParserState::ProcessingAblationDistanceBetweenShotsY,
                b"AblationFrequency" => self.sub_state = ParserState::ProcessingAblationFrequency,
                b"AcquisitionROIID" => self.sub_state = ParserState::ProcessingAcquisitionROIID,
                b"SignalType" => self.sub_state = ParserState::ProcessingSignalType,
                b"DualCountStart" => self.sub_state = ParserState::ProcessingDualCountStart,
                b"DataStartOffset" => self.sub_state = ParserState::ProcessingDataStartOffset,
                b"DataEndOffset" => self.sub_state = ParserState::ProcessingDataEndOffset,
                b"StartTimeStamp" => self.sub_state = ParserState::ProcessingStartTimeStamp,
                b"EndTimeStamp" => self.sub_state = ParserState::ProcessingEndTimeStamp,
                b"AfterAblationImageEndOffset" => self.sub_state = ParserState::ProcessingAfterAblationImageEndOffset,
                b"AfterAblationImageStartOffset" => self.sub_state = ParserState::ProcessingAfterAblationImageStartOffset,
                b"BeforeAblationImageEndOffset" => self.sub_state = ParserState::ProcessingBeforeAblationImageEndOffset,
                b"BeforeAblationImageStartOffset" => self.sub_state = ParserState::ProcessingBeforeAblationImageStartOffset,
                b"ROIStartXPosUm" => self.sub_state = ParserState::ProcessingROIStartXPosUm,
                b"ROIStartYPosUm" => self.sub_state = ParserState::ProcessingROIStartYPosUm,
                b"ROIEndXPosUm" => self.sub_state = ParserState::ProcessingROIEndXPosUm,
                b"ROIEndYPosUm" => self.sub_state = ParserState::ProcessingROIEndYPosUm,
                b"MovementType" => self.sub_state = ParserState::ProcessingMovementType,
                b"SegmentDataFormat" => self.sub_state = ParserState::ProcessingSegmentDataFormat,
                b"ValueBytes" => self.sub_state = ParserState::ProcessingValueBytes,
                b"MaxY" => self.sub_state = ParserState::ProcessingMaxY,
                b"MaxX" => self.sub_state = ParserState::ProcessingMaxX,
                b"PlumeStart" => self.sub_state = ParserState::ProcessingPlumeStart,
                b"PlumeEnd" => self.sub_state = ParserState::ProcessingPlumeEnd,
                b"Template" => self.sub_state = ParserState::ProcessingTemplate,
                b"PanoramaID" => self.sub_state = ParserState::ProcessingPanoramaID,
                b"ROIType" => self.sub_state = ParserState::ProcessingROIType,
                b"SlideXPosUm" => self.sub_state = ParserState::ProcessingSlideXPosUm,
                b"SlideYPosUm" => self.sub_state = ParserState::ProcessingSlideYPosUm,
                b"PanoramaPixelXPos" => self.sub_state = ParserState::ProcessingPanoramaPixelXPos,
                b"PanoramaPixelYPos" => self.sub_state = ParserState::ProcessingPanoramaPixelYPos,
                _ => match std::str::from_utf8(e.local_name()) {
                    Ok(name) => {
                        self.errors
                            .push_back(format!("[Start] Unknown tag name: {}", name));
                    }
                    Err(error) => {
                        println!("Failed to convert tag name: {}", error);

                        self.state = ParserState::FatalError
                    }
                },
            },
            Event::End(e) => match e.local_name() {
                b"Panorama" => {
                    let panorama = self.current_panorama.take().unwrap();
                    self.current_mcd.as_mut().unwrap().panoramas.push(panorama);

                    self.state = ParserState::Processing
                }
                b"AcquisitionChannel" => {
                    let acquisition_channel = self.current_acquisition_channel.take().unwrap();
                    self.current_mcd
                        .as_mut()
                        .unwrap()
                        .acquisition_channels
                        .push(acquisition_channel);

                    self.state = ParserState::Processing
                }
                b"Acquisition" => {
                    let acquisition = self.current_acquisition.take().unwrap();
                    self.current_mcd.as_mut().unwrap().acquisitions.push(acquisition);

                    self.state = ParserState::Processing
                }
                b"AcquisitionROI" => {
                    let acquisition_roi = self.current_acquisition_roi.take().unwrap();
                    self.current_mcd.as_mut().unwrap().acquisition_rois.push(acquisition_roi);

                    self.state = ParserState::Processing
                }
                b"ROIPoint" => {
                    let roi_point = self.current_roi_point.take().unwrap();
                    self.current_mcd.as_mut().unwrap().roi_points.push(roi_point);

                    self.state = ParserState::Processing
                }
                b"MCDSchema" => self.state = ParserState::Finished,
                _ => match std::str::from_utf8(e.local_name()) {
                    Ok(name) => {
                        self.errors
                            .push_back(format!("[End] Unknown tag name: {}", name));
                    }
                    Err(error) => {
                        println!("Failed to convert tag name: {}", error);

                        self.state = ParserState::FatalError
                    }
                },
            },

            Event::Text(e) => {
                match self.state {
                    ParserState::ProcessingSlide => {
                        let ref mut slide = self.current_mcd.as_mut().unwrap().slide;

                        match self.sub_state {
                            ParserState::ProcessingID => {
                                slide.id = Some(
                                    std::str::from_utf8(&e.unescaped().unwrap())
                                        .unwrap()
                                        .to_owned(),
                                )
                            }
                            ParserState::ProcessingUID => {
                                slide.uid = Some(
                                    std::str::from_utf8(&e.unescaped().unwrap())
                                        .unwrap()
                                        .to_owned(),
                                )
                            }
                            ParserState::ProcessingDescription => {
                                slide.description = Some(
                                    std::str::from_utf8(&e.unescaped().unwrap())
                                        .unwrap()
                                        .to_owned(),
                                )
                            }
                            ParserState::ProcessingFilename => {
                                slide.filename = Some(
                                    std::str::from_utf8(&e.unescaped().unwrap())
                                        .unwrap()
                                        .to_owned(),
                                )
                            }
                            ParserState::ProcessingSlideType => {
                                slide.slide_type = Some(
                                    std::str::from_utf8(&e.unescaped().unwrap())
                                        .unwrap()
                                        .to_owned(),
                                )
                            }
                            ParserState::ProcessingWidthUm => {
                                slide.width_um = Some(
                                    std::str::from_utf8(&e.unescaped().unwrap())
                                        .unwrap()
                                        .parse()
                                        .unwrap(),
                                )
                            }
                            ParserState::ProcessingHeightUm => {
                                slide.height_um = Some(
                                    std::str::from_utf8(&e.unescaped().unwrap())
                                        .unwrap()
                                        .parse()
                                        .unwrap(),
                                )
                            }
                            ParserState::ProcessingImageStartOffset => {
                                slide.image_start_offset = Some(
                                    std::str::from_utf8(&e.unescaped().unwrap())
                                        .unwrap()
                                        .parse()
                                        .unwrap(),
                                )
                            }
                            ParserState::ProcessingImageEndOffset => {
                                slide.image_end_offset = Some(
                                    std::str::from_utf8(&e.unescaped().unwrap())
                                        .unwrap()
                                        .parse()
                                        .unwrap(),
                                )
                            }
                            ParserState::ProcessingImageFile => {
                                slide.image_file = Some(
                                    std::str::from_utf8(&e.unescaped().unwrap())
                                        .unwrap()
                                        .to_owned(),
                                )
                            }
                            ParserState::ProcessingSwVersion => {
                                slide.sw_version = Some(
                                    std::str::from_utf8(&e.unescaped().unwrap())
                                        .unwrap()
                                        .to_owned(),
                                )
                            }
                            _ => {}
                        }

                        self.sub_state = ParserState::Processing
                    }

                    ParserState::ProcessingPanorama => {
                        let ref mut panorama = self.current_panorama.as_mut().unwrap();

                        match self.sub_state {
                            ParserState::ProcessingID => {
                                panorama.id = Some(
                                    std::str::from_utf8(&e.unescaped().unwrap())
                                        .unwrap()
                                        .to_owned(),
                                )
                            }
                            ParserState::ProcessingSlideID => {
                                panorama.slide_id = Some(
                                    std::str::from_utf8(&e.unescaped().unwrap())
                                        .unwrap()
                                        .to_owned(),
                                )
                            }
                            ParserState::ProcessingDescription => {
                                panorama.description = Some(
                                    std::str::from_utf8(&e.unescaped().unwrap())
                                        .unwrap()
                                        .to_owned(),
                                )
                            }
                            ParserState::ProcessingSlideX1PosUm => {
                                panorama.slide_x1_pos_um = Some(
                                    std::str::from_utf8(&e.unescaped().unwrap())
                                        .unwrap()
                                        .parse()
                                        .unwrap(),
                                )
                            }
                            ParserState::ProcessingSlideY1PosUm => {
                                panorama.slide_y1_pos_um = Some(
                                    std::str::from_utf8(&e.unescaped().unwrap())
                                        .unwrap()
                                        .parse()
                                        .unwrap(),
                                )
                            }
                            ParserState::ProcessingSlideX2PosUm => {
                                panorama.slide_x2_pos_um = Some(
                                    std::str::from_utf8(&e.unescaped().unwrap())
                                        .unwrap()
                                        .parse()
                                        .unwrap(),
                                )
                            }
                            ParserState::ProcessingSlideY2PosUm => {
                                panorama.slide_y2_pos_um = Some(
                                    std::str::from_utf8(&e.unescaped().unwrap())
                                        .unwrap()
                                        .parse()
                                        .unwrap(),
                                )
                            }
                            ParserState::ProcessingSlideX3PosUm => {
                                panorama.slide_x3_pos_um = Some(
                                    std::str::from_utf8(&e.unescaped().unwrap())
                                        .unwrap()
                                        .parse()
                                        .unwrap(),
                                )
                            }
                            ParserState::ProcessingSlideY3PosUm => {
                                panorama.slide_y3_pos_um = Some(
                                    std::str::from_utf8(&e.unescaped().unwrap())
                                        .unwrap()
                                        .parse()
                                        .unwrap(),
                                )
                            }
                            ParserState::ProcessingSlideX4PosUm => {
                                panorama.slide_x4_pos_um = Some(
                                    std::str::from_utf8(&e.unescaped().unwrap())
                                        .unwrap()
                                        .parse()
                                        .unwrap(),
                                )
                            }
                            ParserState::ProcessingSlideY4PosUm => {
                                panorama.slide_y4_pos_um = Some(
                                    std::str::from_utf8(&e.unescaped().unwrap())
                                        .unwrap()
                                        .parse()
                                        .unwrap(),
                                )
                            }
                            ParserState::ProcessingImageStartOffset => {
                                panorama.image_start_offset = Some(
                                    std::str::from_utf8(&e.unescaped().unwrap())
                                        .unwrap()
                                        .parse()
                                        .unwrap(),
                                )
                            }
                            ParserState::ProcessingImageEndOffset => {
                                panorama.image_end_offset = Some(
                                    std::str::from_utf8(&e.unescaped().unwrap())
                                        .unwrap()
                                        .parse()
                                        .unwrap(),
                                )
                            }
                            ParserState::ProcessingPixelWidth => {
                                panorama.pixel_width = Some(
                                    std::str::from_utf8(&e.unescaped().unwrap())
                                        .unwrap()
                                        .parse()
                                        .unwrap(),
                                )
                            }
                            ParserState::ProcessingPixelHeight => {
                                panorama.pixel_height = Some(
                                    std::str::from_utf8(&e.unescaped().unwrap())
                                        .unwrap()
                                        .parse()
                                        .unwrap(),
                                )
                            }
                            ParserState::ProcessingImageFormat => {
                                match std::str::from_utf8(&e.unescaped().unwrap()).as_ref() {
                                    Ok(&"PNG") => panorama.image_format = Some(ImageFormat::Png),
                                    _ => {}
                                }
                            }
                            ParserState::ProcessingPixelScaleCoef => {
                                panorama.pixel_scale_coef = Some(
                                    std::str::from_utf8(&e.unescaped().unwrap())
                                        .unwrap()
                                        .parse()
                                        .unwrap(),
                                )
                            }
                            _ => {}
                        }

                        self.sub_state = ParserState::Processing
                    }

                    ParserState::ProcessingAcquisitionChannel => {
                        let ref mut acquisition_channel =
                            self.current_acquisition_channel.as_mut().unwrap();
                        let unprocessed_text = &e.unescaped().unwrap();
                        let text = std::str::from_utf8(unprocessed_text).unwrap();

                        match self.sub_state {
                            ParserState::ProcessingID => {
                                acquisition_channel.id = Some(text.to_owned())
                            }
                            ParserState::ProcessingChannelName => {
                                acquisition_channel.channel_name = Some(text.to_owned())
                            }
                            ParserState::ProcessingOrderNumber => {
                                acquisition_channel.order_number = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingAcquisitionID => {
                                acquisition_channel.acquisition_id = Some(text.to_owned())
                            }
                            ParserState::ProcessingChannelLabel => {
                                acquisition_channel.channel_label = Some(text.to_owned())
                            }
                            _ => {}
                        }

                        self.sub_state = ParserState::Processing
                    }

                    ParserState::ProcessingAcquisition => {
                        let ref mut acquisition = self.current_acquisition.as_mut().unwrap();
                        let unprocessed_text = &e.unescaped().unwrap();
                        let text = std::str::from_utf8(unprocessed_text).unwrap();

                        match self.sub_state {
                            ParserState::ProcessingID => acquisition.id = Some(text.to_owned()),
                            ParserState::ProcessingDescription => {
                                acquisition.description = Some(text.to_owned())
                            }
                            ParserState::ProcessingAblationPower => {
                                acquisition.ablation_power = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingAblationDistanceBetweenShotsX => {
                                acquisition.ablation_distance_between_shots_x =
                                    Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingAblationDistanceBetweenShotsY => {
                                acquisition.ablation_distance_between_shots_y =
                                    Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingAblationFrequency => {
                                acquisition.ablation_frequency =
                                    Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingAcquisitionROIID => {
                                acquisition.acquisition_roi_id =
                                    Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingOrderNumber => {
                                acquisition.order_number =
                                    Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingSignalType => {
                                acquisition.signal_type =
                                    Some(text.to_owned())
                            }
                            ParserState::ProcessingDualCountStart => {
                                acquisition.dual_count_start =
                                    Some(text.to_owned())
                            }
                            ParserState::ProcessingDataStartOffset => {
                                acquisition.data_start_offset =
                                    Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingDataEndOffset => {
                                acquisition.data_end_offset =
                                    Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingStartTimeStamp => {
                                acquisition.start_timestamp =
                                    Some(text.to_owned())
                            }
                            ParserState::ProcessingEndTimeStamp => {
                                acquisition.end_timestamp =
                                    Some(text.to_owned())
                            }
                            ParserState::ProcessingAfterAblationImageEndOffset => {
                                acquisition.after_ablation_image_end_offset =
                                    Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingAfterAblationImageStartOffset => {
                                acquisition.after_ablation_image_start_offset =
                                    Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingBeforeAblationImageEndOffset => {
                                acquisition.before_ablation_image_end_offset =
                                    Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingBeforeAblationImageStartOffset => {
                                acquisition.before_ablation_image_start_offset =
                                    Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingROIStartXPosUm => {
                                acquisition.roi_start_x_pos_um =
                                    Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingROIStartYPosUm => {
                                acquisition.roi_start_y_pos_um =
                                    Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingROIEndXPosUm => {
                                acquisition.roi_end_x_pos_um =
                                    Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingROIEndYPosUm => {
                                acquisition.roi_end_y_pos_um =
                                    Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingMovementType => {
                                acquisition.movement_type =
                                    Some(text.to_owned())
                            }
                            ParserState::ProcessingSegmentDataFormat => {
                                match &text as &str {
                                    "Float" =>  acquisition.segment_data_format =
                                            Some(DataFormat::Float),
                                    _ => {}
                                }
                                
                            }
                            ParserState::ProcessingValueBytes => {
                                acquisition.value_bytes =
                                    Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingMaxX => {
                                acquisition.max_x =
                                    Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingMaxY => {
                                acquisition.max_y =
                                    Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingPlumeStart => {
                                acquisition.plume_start =
                                    Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingPlumeEnd => {
                                acquisition.plume_end =
                                    Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingTemplate => {
                                acquisition.template =
                                    Some(text.to_owned())
                            }
                            _ => {}
                        }

                        self.sub_state = ParserState::Processing
                    }


                    ParserState::ProcessingAcquisitionROI => {
                        let ref mut acquisition_roi =
                            self.current_acquisition_roi.as_mut().unwrap();
                        let unprocessed_text = &e.unescaped().unwrap();
                        let text = std::str::from_utf8(unprocessed_text).unwrap();

                        match self.sub_state {
                            ParserState::ProcessingID => {
                                acquisition_roi.id = Some(text.to_owned())
                            }
                            ParserState::ProcessingPanoramaID => {
                                acquisition_roi.panorama_id = Some(text.to_owned())
                            }
                            ParserState::ProcessingROIType => {
                                match &text as &str {
                                    "Acquisition" =>  acquisition_roi.roi_type =
                                            Some(ROIType::Acquisition),
                                    _ => {}
                                }
                            }
                            _ => {}
                        }

                        self.sub_state = ParserState::Processing
                    }

                    ParserState::ProcessingROIPoint => {
                        let ref mut roi_point =
                            self.current_roi_point.as_mut().unwrap();
                        let unprocessed_text = &e.unescaped().unwrap();
                        let text = std::str::from_utf8(unprocessed_text).unwrap();

                        match self.sub_state {
                            ParserState::ProcessingID => {
                                roi_point.id = Some(text.to_owned())
                            }
                            ParserState::ProcessingAcquisitionROIID => {
                                roi_point.acquisition_roi_id = Some(text.to_owned())
                            }
                            ParserState::ProcessingOrderNumber => {
                                roi_point.order_number = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingSlideXPosUm => {
                                roi_point.slide_x_pos_um = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingSlideYPosUm => {
                                roi_point.slide_y_pos_um = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingPanoramaPixelXPos => {
                                roi_point.panorama_pixel_x_pos = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingPanoramaPixelYPos => {
                                roi_point.panorama_pixel_y_pos = Some(text.parse().unwrap())
                            }
                            _ => {}
                        }

                        self.sub_state = ParserState::Processing
                    }
                    _ => {}
                }
                //println!("text: {}", std::str::from_utf8(&e.unescaped()?)?);
            }

            Event::Eof => self.state = ParserState::Finished,

            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::time::Instant;
    use quick_xml::events::{attributes::Attribute, Event};
    use std::fs::File;
    use std::io::prelude::*;
    use std::io::SeekFrom;

    use std::convert::TryInto;

    #[test]
    fn it_works() {

        let mut parser = MCDParser::new();let start = Instant::now();
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

        println!("{:?}", parser.current_mcd.as_ref().unwrap().slide);
        println!("{:?}", parser.current_mcd.as_ref().unwrap().panoramas);
        println!(
            "{:?}",
            parser.current_mcd.as_ref().unwrap().roi_points[0]
        );
        std::fs::write("tmp.xml", combined_xml).expect("Unable to write file");

        std::fs::write("tmp.png", parser.current_mcd.as_ref().unwrap().panoramas[0].get_image(&mut file).unwrap()).expect("Unable to write file");

        println!("Time elapsed when parsing is: {:?}", duration);

        //println!("{}", combined_xml);

        assert_eq!(2 + 2, 4);
    }
}
