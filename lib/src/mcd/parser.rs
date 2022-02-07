use std::collections::HashMap;

use quick_xml::events::Event;

use crate::{
    acquisition::{DataFormat, ProfilingType},
    calibration::{Calibration, CalibrationChannel, CalibrationFinal, CalibrationParams},
    panorama::PanoramaType,
    slide::{SlideFiducialMarks, SlideProfile},
};

use super::{
    xml_types::{CalibrationFinalXML, CalibrationParamsXML, CalibrationXML},
    Acquisition, AcquisitionChannel, AcquisitionChannelXML, AcquisitionROI, AcquisitionXML,
    CalibrationChannelXML, ImageFormat, Panorama, PanoramaXML, ROIPoint, ROIType,
    SlideFiducialMarksXML, SlideProfileXML, SlideXML, MCD,
};

use std::io::prelude::*;

#[derive(Clone, Copy, Debug)]
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
    ProcessingEnergyDb,
    ProcessingFrequency,
    ProcessingFMarkSlideLength,
    ProcessingFMarkSlideThickness,
    ProcessingName,
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
    ProcessingType,
    ProcessingIsLocked,
    ProcessingRotationAngle,
    ProcessingCalibration,
    ProcessingCalibrationParams,
    ProcessingCalibrationChannel,
    ProcessingCalibrationFinal,
    ProcessingTimeStamp,
    ProcessingOptimalDetectorVoltageStart,
    ProcessingOptimalDetectorVoltageEnd,
    ProcessingOptimalDetectorDualCoefficientStart,
    ProcessingOptimalDetectorDualCoefficientEnd,
    ProcessingOptimalHelium,
    ProcessingTransientStart,
    ProcessingTransientCrossTalk1,
    ProcessingTransientCrossTalk2,
    ProcessingReferenceEnergy,
    ProcessingMaximumEnergy,
    ProcessingCalibrationID,
    ProcessingOptimalDetectorVoltage,
    ProcessingOptimalDetectorDualCoefficient,
    ProcessingOptimalMakeupGas,
    ProcessingOptimalCurrent,
    ProcessingOptimalX,
    ProcessingOptimalY,
    ProcessingMeanDuals,
    ProcessingSlideFiducialMarks,
    ProcessingSlideProfile,
    ProcessingCoordinateX,
    ProcessingCoordinateY,
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
    ProcessingProfilingType,
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
    #[allow(dead_code)]
    Error,
    FatalError, // Must stop here
    Finished,
}

pub struct MCDParser<T: Seek + BufRead> {
    pub(crate) current_mcd: Option<MCD<T>>,

    state: ParserState,
    sub_state: ParserState,
    //pub(super) history: Vec<String>,
    errors: std::collections::VecDeque<String>,

    panoramas: HashMap<u16, Panorama<T>>,
    calibration_finals: HashMap<u16, CalibrationFinal>,
    calibration_params: HashMap<u16, CalibrationParams>,
    calibration_channels: HashMap<u16, CalibrationChannel>,
    calibrations: HashMap<u16, Calibration>,
    slide_fiducal_marks: HashMap<u16, SlideFiducialMarks>,
    slide_profiles: HashMap<u16, SlideProfile>,
    acquisitions: HashMap<u16, Acquisition<T>>,
    acquisition_channels: Vec<AcquisitionChannel>,
    acquisition_rois: Vec<AcquisitionROI>,

    roi_points: Vec<ROIPoint>,

    current_slide: Option<SlideXML>,
    current_panorama: Option<PanoramaXML>,
    current_calibration_final: Option<CalibrationFinalXML>,
    current_calibration_params: Option<CalibrationParamsXML>,
    current_calibration_channel: Option<CalibrationChannelXML>,
    current_calibration: Option<CalibrationXML>,
    current_slide_fiducial_marks: Option<SlideFiducialMarksXML>,
    current_slide_profile: Option<SlideProfileXML>,
    current_acquisition_channel: Option<AcquisitionChannelXML>,
    current_acquisition: Option<AcquisitionXML>,
    current_acquisition_roi: Option<AcquisitionROI>,
    current_roi_point: Option<ROIPoint>,
}

impl<T: Seek + BufRead> MCDParser<T> {
    pub fn new(mcd: MCD<T>) -> MCDParser<T> {
        MCDParser {
            current_mcd: Some(mcd),
            state: ParserState::Start,
            sub_state: ParserState::Start,
            errors: std::collections::VecDeque::new(),

            panoramas: HashMap::new(),
            calibration_finals: HashMap::new(),
            calibration_params: HashMap::new(),
            calibration_channels: HashMap::new(),
            calibrations: HashMap::new(),
            slide_fiducal_marks: HashMap::new(),
            slide_profiles: HashMap::new(),
            acquisitions: HashMap::new(),
            acquisition_channels: Vec::new(),
            acquisition_rois: Vec::new(),

            // TODO: Do we need this?
            roi_points: Vec::new(),

            current_slide: None,
            current_panorama: None,
            current_calibration_final: None,
            current_calibration_params: None,
            current_calibration_channel: None,
            current_calibration: None,
            current_slide_fiducial_marks: None,
            current_slide_profile: None,
            current_acquisition_channel: None,
            current_acquisition: None,
            current_acquisition_roi: None,
            current_roi_point: None,
        }
    }

    pub fn mcd(&mut self) -> MCD<T> {
        let mut mcd = self
            .current_mcd
            .take()
            .expect("Can't call get_mcd() when the parse hasn't been run");

        let reader = mcd.reader().clone();

        // Add the channels to the corresponding acquisition
        for channel in self.acquisition_channels.drain(0..) {
            let acquisition = self
                .acquisitions
                .get_mut(&channel.acquisition_id())
                .unwrap_or_else(|| panic!("Missing AcquisitionID {}", channel.acquisition_id()));
            acquisition.channels_mut().push(channel);
        }

        // Create map with Arc for sharing pointers with Panorama
        let mut acquisitions = HashMap::new();
        for (id, mut acquisition) in self.acquisitions.drain() {
            acquisition.reader = Some(reader.clone());
            acquisition.fix_roi_start_pos();

            acquisitions.insert(id, acquisition);
        }

        // Add acquisition to panorama
        for roi in &self.acquisition_rois {
            let acquisition = acquisitions
                .remove(roi.id.as_ref().expect("Must have ID for AcquisitionROI"))
                .expect("Should have Acquisition with same ID as AcquisitionROI");

            let panorama = self
                .panoramas
                .get_mut(
                    roi.panorama_id
                        .as_ref()
                        .expect("Must have PanoramaID for AcquisitionROI"),
                )
                .expect("Should have Panorama with same ID as AcquisitionROI");

            panorama
                .acquisitions_mut()
                .insert(acquisition.id(), acquisition);
        }

        for (id, mut panorama) in self.panoramas.drain() {
            //mcd.panoramas.insert(id, panorama);
            let slide_id = panorama.slide_id();

            let slide = mcd
                .slides
                .get_mut(&slide_id)
                .unwrap_or_else(|| panic!("Missing Slide with ID {}", slide_id));
            panorama.reader = Some(reader.clone());

            panorama.fix_image_dimensions();

            slide.panoramas_mut().insert(id, panorama);
        }

        // Update the acquisitions
        //mcd.acquisitions = acquisitions;
        for slide in mcd.slides.values_mut() {
            slide.reader = Some(reader.clone());
        }

        mcd
    }

    pub fn current_state(&self) -> ParserState {
        self.state
    }

    #[allow(dead_code)]
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
    #[allow(dead_code)]
    pub fn pop_error_front(&mut self) -> Option<String> {
        self.errors.pop_front()
    }

    pub fn pop_error_back(&mut self) -> Option<String> {
        self.errors.pop_back()
    }

    pub fn process(&mut self, ev: Event) {
        match &ev {
            Event::Start(e) | Event::Empty(e) => match e.local_name() {
                b"MCDSchema" => {
                    // TODO: get xmlns
                    //self.current_mcd = Some()
                }
                b"Slide" => {
                    //Wself.current_mcd.unwrap().slide = Some(Slide::new());
                    self.current_slide = Some(SlideXML::new());
                    self.state = ParserState::ProcessingSlide
                }
                b"Panorama" => {
                    self.current_panorama = Some(PanoramaXML::new());
                    self.state = ParserState::ProcessingPanorama
                }
                b"CalibrationFinal" => {
                    self.current_calibration_final = Some(CalibrationFinalXML::new());
                    self.state = ParserState::ProcessingCalibrationFinal
                }
                b"CalibrationParams" => {
                    self.current_calibration_params = Some(CalibrationParamsXML::new());
                    self.state = ParserState::ProcessingCalibrationParams
                }
                b"CalibrationChannel" => {
                    self.current_calibration_channel = Some(CalibrationChannelXML::new());
                    self.state = ParserState::ProcessingCalibrationChannel
                }
                b"Calibration" => {
                    self.current_calibration = Some(CalibrationXML::new());
                    self.state = ParserState::ProcessingCalibration
                }
                b"SlideFiducialMarks" => {
                    self.current_slide_fiducial_marks = Some(SlideFiducialMarksXML::new());
                    self.state = ParserState::ProcessingSlideFiducialMarks
                }
                b"SlideProfile" => {
                    self.current_slide_profile = Some(SlideProfileXML::new());
                    self.state = ParserState::ProcessingSlideProfile
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
                    self.current_acquisition_channel = Some(AcquisitionChannelXML::new());
                    self.state = ParserState::ProcessingAcquisitionChannel
                }
                b"Acquisition" => {
                    self.current_acquisition = Some(AcquisitionXML::new());
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
                b"EnergyDb" => self.sub_state = ParserState::ProcessingEnergyDb,
                b"Frequency" => self.sub_state = ParserState::ProcessingFrequency,
                b"FMarkSlideLength" => self.sub_state = ParserState::ProcessingFMarkSlideLength,
                b"FMarkSlideThickness" => {
                    self.sub_state = ParserState::ProcessingFMarkSlideThickness
                }
                b"Name" => self.sub_state = ParserState::ProcessingName,
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
                b"Type" => self.sub_state = ParserState::ProcessingType,
                b"IsLocked" => self.sub_state = ParserState::ProcessingIsLocked,
                b"RotationAngle" => self.sub_state = ParserState::ProcessingRotationAngle,
                b"TimeStamp" | b"Timestamp" => self.sub_state = ParserState::ProcessingTimeStamp,
                b"OptimalDetectorVoltageStart" => {
                    self.sub_state = ParserState::ProcessingOptimalDetectorVoltageStart
                }
                b"OptimalDetectorVoltageEnd" => {
                    self.sub_state = ParserState::ProcessingOptimalDetectorVoltageEnd
                }
                b"OptimalDetectorDualCoefficientStart" => {
                    self.sub_state = ParserState::ProcessingOptimalDetectorDualCoefficientStart
                }
                b"OptimalDetectorDualCoefficientEnd" => {
                    self.sub_state = ParserState::ProcessingOptimalDetectorDualCoefficientEnd
                }
                b"OptimalHelium" => self.sub_state = ParserState::ProcessingOptimalHelium,
                b"TransientStart" => self.sub_state = ParserState::ProcessingTransientStart,
                b"TransientCrossTalk1" => {
                    self.sub_state = ParserState::ProcessingTransientCrossTalk1
                }
                b"TransientCrossTalk2" => {
                    self.sub_state = ParserState::ProcessingTransientCrossTalk2
                }
                b"ReferenceEnergy" => self.sub_state = ParserState::ProcessingReferenceEnergy,
                b"MaximumEnergy" => self.sub_state = ParserState::ProcessingMaximumEnergy,
                b"CalibrationID" => self.sub_state = ParserState::ProcessingCalibrationID,
                b"OptimalDetectorVoltage" => {
                    self.sub_state = ParserState::ProcessingOptimalDetectorVoltage
                }
                b"OptimalDetectorDualCoefficient" => {
                    self.sub_state = ParserState::ProcessingOptimalDetectorDualCoefficient
                }
                b"OptimalMakeupGas" => self.sub_state = ParserState::ProcessingOptimalMakeupGas,
                b"OptimalCurrent" => self.sub_state = ParserState::ProcessingOptimalCurrent,
                b"OptimalX" => self.sub_state = ParserState::ProcessingOptimalX,
                b"OptimalY" => self.sub_state = ParserState::ProcessingOptimalY,
                b"MeanDuals" => self.sub_state = ParserState::ProcessingMeanDuals,
                b"CoordinateX" => self.sub_state = ParserState::ProcessingCoordinateX,
                b"CoordinateY" => self.sub_state = ParserState::ProcessingCoordinateY,
                b"ChannelName" => self.sub_state = ParserState::ProcessingChannelName,
                b"OrderNumber" => self.sub_state = ParserState::ProcessingOrderNumber,
                b"AcquisitionID" => self.sub_state = ParserState::ProcessingAcquisitionID,
                b"ChannelLabel" => {
                    self.sub_state = ParserState::ProcessingChannelLabel;
                }
                b"AblationPower" => self.sub_state = ParserState::ProcessingAblationPower,
                b"AblationDistanceBetweenShotsX" => {
                    self.sub_state = ParserState::ProcessingAblationDistanceBetweenShotsX
                }
                b"AblationDistanceBetweenShotsY" => {
                    self.sub_state = ParserState::ProcessingAblationDistanceBetweenShotsY
                }
                b"AblationFrequency" => self.sub_state = ParserState::ProcessingAblationFrequency,
                b"AcquisitionROIID" => self.sub_state = ParserState::ProcessingAcquisitionROIID,
                b"SignalType" => self.sub_state = ParserState::ProcessingSignalType,
                b"DualCountStart" => self.sub_state = ParserState::ProcessingDualCountStart,
                b"DataStartOffset" => self.sub_state = ParserState::ProcessingDataStartOffset,
                b"DataEndOffset" => self.sub_state = ParserState::ProcessingDataEndOffset,
                b"StartTimeStamp" => self.sub_state = ParserState::ProcessingStartTimeStamp,
                b"EndTimeStamp" => self.sub_state = ParserState::ProcessingEndTimeStamp,
                b"AfterAblationImageEndOffset" => {
                    self.sub_state = ParserState::ProcessingAfterAblationImageEndOffset
                }
                b"AfterAblationImageStartOffset" => {
                    self.sub_state = ParserState::ProcessingAfterAblationImageStartOffset
                }
                b"BeforeAblationImageEndOffset" => {
                    self.sub_state = ParserState::ProcessingBeforeAblationImageEndOffset
                }
                b"BeforeAblationImageStartOffset" => {
                    self.sub_state = ParserState::ProcessingBeforeAblationImageStartOffset
                }
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
                b"ProfilingType" => self.sub_state = ParserState::ProcessingProfilingType,
                b"PanoramaID" => self.sub_state = ParserState::ProcessingPanoramaID,
                b"ROIType" => {
                    // In version 2 of XSD there are empty ROIType tags, so only trigger processing of ROIType
                    // if we have a start tag (not an empty tag)
                    match &ev {
                        Event::Start(_e) => self.sub_state = ParserState::ProcessingROIType,
                        _ => self.sub_state = ParserState::Processing,
                    };
                }
                b"SlideXPosUm" => self.sub_state = ParserState::ProcessingSlideXPosUm,
                b"SlideYPosUm" => self.sub_state = ParserState::ProcessingSlideYPosUm,
                b"PanoramaPixelXPos" => self.sub_state = ParserState::ProcessingPanoramaPixelXPos,
                b"PanoramaPixelYPos" => self.sub_state = ParserState::ProcessingPanoramaPixelYPos,
                _ => match std::str::from_utf8(e.local_name()) {
                    Ok(name) => {
                        self.errors
                            .push_back(format!("[Start] Unknown tag name: {}", name));

                        panic!("[Start] Unknown tag name: {}", name);
                    }
                    Err(error) => {
                        println!("Failed to convert tag name: {}", error);

                        self.state = ParserState::FatalError
                    }
                },
            },
            Event::End(e) => match e.local_name() {
                b"Slide" => {
                    let slide = self.current_slide.take().unwrap();
                    self.current_mcd
                        .as_mut()
                        .unwrap()
                        .slides
                        .insert(slide.id.unwrap(), slide.into());

                    self.state = ParserState::Processing
                }
                b"Panorama" => {
                    let panorama = self.current_panorama.take().unwrap();
                    let panorama_id = panorama.id.as_ref().unwrap();
                    self.panoramas.insert(*panorama_id, panorama.into());

                    self.state = ParserState::Processing
                }
                b"CalibrationFinal" => {
                    let calibration_final = self.current_calibration_final.take().unwrap();
                    let calibration_final_id = calibration_final.id.as_ref().unwrap();
                    self.calibration_finals
                        .insert(*calibration_final_id, calibration_final.into());

                    self.state = ParserState::Processing
                }
                b"CalibrationParams" => {
                    let calibration_params = self.current_calibration_params.take().unwrap();
                    let calibration_params_id = calibration_params.calibration_id.as_ref().unwrap();
                    self.calibration_params
                        .insert(*calibration_params_id, calibration_params.into());

                    self.state = ParserState::Processing
                }
                b"CalibrationChannel" => {
                    let calibration_channel = self.current_calibration_channel.take().unwrap();
                    let calibration_channel_id = calibration_channel.id.as_ref().unwrap();
                    self.calibration_channels
                        .insert(*calibration_channel_id, calibration_channel.into());

                    self.state = ParserState::Processing
                }
                b"Calibration" => {
                    let calibration = self.current_calibration.take().unwrap();
                    let calibration_id = calibration.id.as_ref().unwrap();
                    self.calibrations
                        .insert(*calibration_id, calibration.into());

                    self.state = ParserState::Processing
                }
                b"SlideFiducialMarks" => {
                    let slide_fiducal_marks = self.current_slide_fiducial_marks.take().unwrap();
                    let id = slide_fiducal_marks.id.as_ref().unwrap();
                    self.slide_fiducal_marks
                        .insert(*id, slide_fiducal_marks.into());

                    self.state = ParserState::Processing
                }
                b"SlideProfile" => {
                    let slide_profile = self.current_slide_profile.take().unwrap();
                    let id = slide_profile.id.as_ref().unwrap();
                    self.slide_profiles.insert(*id, slide_profile.into());

                    self.state = ParserState::Processing
                }
                b"AcquisitionChannel" => {
                    let acquisition_channel = self.current_acquisition_channel.take().unwrap();
                    self.acquisition_channels.push(acquisition_channel.into());

                    self.state = ParserState::Processing
                }
                b"Acquisition" => {
                    let acquisition = self.current_acquisition.take().unwrap();
                    let acquisition_id = acquisition.id.as_ref().unwrap();

                    //self.current_mcd.as_mut().unwrap().acquisition_order.push(acquisition_id.clone());
                    self.acquisitions
                        .insert(*acquisition_id, acquisition.into());

                    self.state = ParserState::Processing
                }
                b"AcquisitionROI" => {
                    let acquisition_roi = self.current_acquisition_roi.take().unwrap();
                    self.acquisition_rois.push(acquisition_roi);

                    self.state = ParserState::Processing
                }
                b"ROIPoint" => {
                    let roi_point = self.current_roi_point.take().unwrap();
                    self.roi_points.push(roi_point);

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
                        let slide = self.current_slide.as_mut().unwrap();
                        let unprocessed_text = &e.unescaped().unwrap();
                        let text = std::str::from_utf8(unprocessed_text).unwrap();

                        match self.sub_state {
                            ParserState::ProcessingID => slide.id = Some(text.parse().unwrap()),
                            ParserState::ProcessingUID => slide.uid = Some(text.parse().unwrap()),
                            ParserState::ProcessingDescription => {
                                slide.description = Some(text.to_string())
                            }
                            ParserState::ProcessingFilename => {
                                slide.filename = Some(text.to_string())
                            }
                            ParserState::ProcessingSlideType => {
                                slide.slide_type = Some(text.to_string())
                            }
                            ParserState::ProcessingWidthUm => {
                                slide.width_um = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingHeightUm => {
                                slide.height_um = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingImageStartOffset => {
                                slide.image_start_offset = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingImageEndOffset => {
                                slide.image_end_offset = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingImageFile => {
                                slide.image_file = Some(text.to_string())
                            }
                            ParserState::ProcessingEnergyDb => {
                                slide.energy_db = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingFrequency => {
                                slide.frequency = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingFMarkSlideLength => {
                                slide.fmark_slide_length = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingFMarkSlideThickness => {
                                slide.fmark_slide_thickness = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingName => slide.name = Some(text.to_string()),
                            ParserState::ProcessingSwVersion => {
                                slide.sw_version = Some(text.to_string())
                            }
                            _ => {}
                        }

                        self.sub_state = ParserState::Processing
                    }

                    ParserState::ProcessingPanorama => {
                        let panorama = self.current_panorama.as_mut().unwrap();
                        let unprocessed_text = &e.unescaped().unwrap();
                        let text = std::str::from_utf8(unprocessed_text).unwrap();

                        match self.sub_state {
                            ParserState::ProcessingID => panorama.id = Some(text.parse().unwrap()),
                            ParserState::ProcessingSlideID => {
                                panorama.slide_id = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingDescription => {
                                panorama.description = Some(text.to_string())
                            }
                            ParserState::ProcessingSlideX1PosUm => {
                                panorama.slide_x1_pos_um = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingSlideY1PosUm => {
                                panorama.slide_y1_pos_um = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingSlideX2PosUm => {
                                panorama.slide_x2_pos_um = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingSlideY2PosUm => {
                                panorama.slide_y2_pos_um = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingSlideX3PosUm => {
                                panorama.slide_x3_pos_um = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingSlideY3PosUm => {
                                panorama.slide_y3_pos_um = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingSlideX4PosUm => {
                                panorama.slide_x4_pos_um = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingSlideY4PosUm => {
                                panorama.slide_y4_pos_um = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingImageStartOffset => {
                                panorama.image_start_offset = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingImageEndOffset => {
                                panorama.image_end_offset = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingPixelWidth => {
                                panorama.pixel_width = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingPixelHeight => {
                                panorama.pixel_height = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingImageFormat => {
                                match std::str::from_utf8(&e.unescaped().unwrap()).as_ref() {
                                    Ok(&"PNG") => panorama.image_format = Some(ImageFormat::Png),
                                    Ok(_) => todo!(),
                                    Err(_) => todo!(),
                                }
                            }
                            ParserState::ProcessingPixelScaleCoef => {
                                panorama.pixel_scale_coef = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingType => match text {
                                "Default" => {
                                    panorama.panorama_type = Some(PanoramaType::Default);
                                }
                                "Imported" => {
                                    panorama.panorama_type = Some(PanoramaType::Imported);
                                }
                                "Instrument" => {
                                    panorama.panorama_type = Some(PanoramaType::Instrument);
                                }
                                _ => {
                                    panic!("Unknown panorama type: {}", text);
                                }
                            },
                            ParserState::ProcessingIsLocked => {
                                panorama.is_locked = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingRotationAngle => {
                                panorama.rotation_angle = Some(text.parse().unwrap())
                            }
                            ParserState::Processing => {}
                            _ => {
                                panic!(
                                    "Unknown sub state {:?} in state {:?} with event {:?}",
                                    self.sub_state, self.state, e
                                );
                            }
                        }

                        self.sub_state = ParserState::Processing
                    }

                    ParserState::ProcessingCalibrationFinal => {
                        let calibration_final = self.current_calibration_final.as_mut().unwrap();
                        let unprocessed_text = &e.unescaped().unwrap();
                        let text = std::str::from_utf8(unprocessed_text).unwrap();

                        match self.sub_state {
                            ParserState::ProcessingID => {
                                calibration_final.id = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingAcquisitionID => {
                                calibration_final.acquisition_id = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingTimeStamp => {
                                calibration_final.time_stamp = Some(text.to_string())
                            }
                            ParserState::ProcessingOptimalDetectorVoltageStart => {
                                calibration_final.optimal_detector_voltage_start =
                                    Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingOptimalDetectorVoltageEnd => {
                                calibration_final.optimal_detector_voltage_end =
                                    Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingOptimalDetectorDualCoefficientStart => {
                                calibration_final.optimal_detector_dual_coefficient_start =
                                    Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingOptimalDetectorDualCoefficientEnd => {
                                calibration_final.optimal_detector_dual_coefficient_end =
                                    Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingOptimalHelium => {
                                calibration_final.optimal_helium = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingTransientStart => {
                                calibration_final.transient_start = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingTransientCrossTalk1 => {
                                calibration_final.transient_cross_talk_1 =
                                    Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingTransientCrossTalk2 => {
                                calibration_final.transient_cross_talk_2 =
                                    Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingReferenceEnergy => {
                                calibration_final.reference_energy = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingMaximumEnergy => {
                                calibration_final.maximum_energy = Some(text.parse().unwrap())
                            }
                            ParserState::Processing => {}
                            _ => {
                                panic!(
                                    "Unknown sub state {:?} in state {:?} with event {:?}",
                                    self.sub_state, self.state, e
                                );
                            }
                        }

                        self.sub_state = ParserState::Processing
                    }

                    ParserState::ProcessingCalibrationParams => {
                        let calibration_params = self.current_calibration_params.as_mut().unwrap();
                        let unprocessed_text = &e.unescaped().unwrap();
                        let text = std::str::from_utf8(unprocessed_text).unwrap();

                        match self.sub_state {
                            ParserState::ProcessingCalibrationID => {
                                calibration_params.calibration_id = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingOptimalDetectorVoltage => {
                                calibration_params.optimal_detector_voltage =
                                    Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingOptimalDetectorDualCoefficient => {
                                calibration_params.optimal_detector_dual_coefficient =
                                    Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingOptimalMakeupGas => {
                                calibration_params.optimal_makeup_gas = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingOptimalCurrent => {
                                calibration_params.optimal_current = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingOptimalX => {
                                calibration_params.optimal_x = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingOptimalY => {
                                calibration_params.optimal_y = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingTransientStart => {
                                calibration_params.transient_start = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingTransientCrossTalk1 => {
                                calibration_params.transient_cross_talk_1 =
                                    Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingTransientCrossTalk2 => {
                                calibration_params.transient_cross_talk_2 =
                                    Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingOptimalHelium => {
                                calibration_params.optimal_helium = Some(text.parse().unwrap())
                            }
                            ParserState::Processing => {}
                            _ => {
                                panic!(
                                    "Unknown sub state {:?} in state {:?} with event {:?}",
                                    self.sub_state, self.state, e
                                );
                            }
                        }

                        self.sub_state = ParserState::Processing
                    }

                    ParserState::ProcessingCalibrationChannel => {
                        let calibration_channel =
                            self.current_calibration_channel.as_mut().unwrap();
                        let unprocessed_text = &e.unescaped().unwrap();
                        let text = std::str::from_utf8(unprocessed_text).unwrap();

                        match self.sub_state {
                            ParserState::ProcessingID => {
                                calibration_channel.id = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingCalibrationID => {
                                calibration_channel.calibration_id = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingName => {
                                calibration_channel.name = Some(text.to_string())
                            }
                            ParserState::ProcessingMeanDuals => {
                                calibration_channel.mean_duals = Some(text.parse().unwrap())
                            }
                            ParserState::Processing => {}
                            _ => {
                                panic!(
                                    "Unknown sub state {:?} in state {:?} with event {:?}",
                                    self.sub_state, self.state, e
                                );
                            }
                        }

                        self.sub_state = ParserState::Processing
                    }

                    ParserState::ProcessingCalibration => {
                        let calibration = self.current_calibration.as_mut().unwrap();
                        let unprocessed_text = &e.unescaped().unwrap();
                        let text = std::str::from_utf8(unprocessed_text).unwrap();

                        match self.sub_state {
                            ParserState::ProcessingID => {
                                calibration.id = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingAcquisitionID => {
                                calibration.acquisition_id = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingTimeStamp => {
                                calibration.time_stamp = Some(text.to_string())
                            }
                            ParserState::Processing => {}
                            _ => {
                                panic!(
                                    "Unknown sub state {:?} in state {:?} with event {:?}",
                                    self.sub_state, self.state, e
                                );
                            }
                        }

                        self.sub_state = ParserState::Processing
                    }

                    ParserState::ProcessingSlideFiducialMarks => {
                        let slide_fiducal_marks =
                            self.current_slide_fiducial_marks.as_mut().unwrap();
                        let unprocessed_text = &e.unescaped().unwrap();
                        let text = std::str::from_utf8(unprocessed_text).unwrap();

                        match self.sub_state {
                            ParserState::ProcessingID => {
                                slide_fiducal_marks.id = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingSlideID => {
                                slide_fiducal_marks.slide_id = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingCoordinateX => {
                                slide_fiducal_marks.coordinate_x = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingCoordinateY => {
                                slide_fiducal_marks.coordinate_y = Some(text.parse().unwrap())
                            }
                            ParserState::Processing => {}
                            _ => {
                                panic!(
                                    "Unknown sub state {:?} in state {:?} with event {:?}",
                                    self.sub_state, self.state, e
                                );
                            }
                        }

                        self.sub_state = ParserState::Processing
                    }

                    ParserState::ProcessingSlideProfile => {
                        let slide_profile = self.current_slide_profile.as_mut().unwrap();
                        let unprocessed_text = &e.unescaped().unwrap();
                        let text = std::str::from_utf8(unprocessed_text).unwrap();

                        match self.sub_state {
                            ParserState::ProcessingID => {
                                slide_profile.id = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingSlideID => {
                                slide_profile.slide_id = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingCoordinateX => {
                                slide_profile.coordinate_x = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingCoordinateY => {
                                slide_profile.coordinate_y = Some(text.parse().unwrap())
                            }
                            ParserState::Processing => {}
                            _ => {
                                panic!(
                                    "Unknown sub state {:?} in state {:?} with event {:?}",
                                    self.sub_state, self.state, e
                                );
                            }
                        }

                        self.sub_state = ParserState::Processing
                    }

                    ParserState::ProcessingAcquisitionChannel => {
                        let acquisition_channel =
                            self.current_acquisition_channel.as_mut().unwrap();
                        let unprocessed_text = &e.unescaped().unwrap();
                        let text = std::str::from_utf8(unprocessed_text).unwrap();

                        match self.sub_state {
                            ParserState::ProcessingID => {
                                acquisition_channel.id = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingChannelName => {
                                acquisition_channel.channel_name = Some(text.to_owned())
                            }
                            ParserState::ProcessingOrderNumber => {
                                acquisition_channel.order_number = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingAcquisitionID => {
                                let value = match text.parse() {
                                    Ok(value) => value,
                                    Err(error) => {
                                        panic!(
                                            "Cannot convert AcquisitionID with text {}. {}",
                                            text, error
                                        );
                                    }
                                };

                                acquisition_channel.acquisition_id = Some(value)
                            }
                            ParserState::ProcessingChannelLabel => {
                                acquisition_channel.channel_label = Some(text.to_owned())
                            }
                            ParserState::Processing => {}
                            _ => {
                                panic!(
                                    "Unknown sub state {:?} in state {:?} with event {:?}",
                                    self.sub_state, self.state, e
                                );
                            }
                        }

                        self.sub_state = ParserState::Processing
                    }

                    ParserState::ProcessingAcquisition => {
                        let acquisition = self.current_acquisition.as_mut().unwrap();
                        let unprocessed_text = &e.unescaped().unwrap();
                        let text = std::str::from_utf8(unprocessed_text).unwrap();

                        match self.sub_state {
                            ParserState::ProcessingID => {
                                acquisition.id = Some(text.parse().unwrap())
                            }
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
                                acquisition.ablation_frequency = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingAcquisitionROIID => {
                                acquisition.acquisition_roi_id = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingOrderNumber => {
                                acquisition.order_number = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingSignalType => {
                                acquisition.signal_type = Some(text.to_owned())
                            }
                            ParserState::ProcessingDualCountStart => {
                                acquisition.dual_count_start = Some(text.to_owned())
                            }
                            ParserState::ProcessingDataStartOffset => {
                                acquisition.data_start_offset = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingDataEndOffset => {
                                acquisition.data_end_offset = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingStartTimeStamp => {
                                acquisition.start_timestamp = Some(text.to_owned())
                            }
                            ParserState::ProcessingEndTimeStamp => {
                                acquisition.end_timestamp = Some(text.to_owned())
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
                                acquisition.roi_start_x_pos_um = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingROIStartYPosUm => {
                                acquisition.roi_start_y_pos_um = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingROIEndXPosUm => {
                                acquisition.roi_end_x_pos_um = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingROIEndYPosUm => {
                                acquisition.roi_end_y_pos_um = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingMovementType => {
                                acquisition.movement_type = Some(text.to_owned())
                            }
                            ParserState::ProcessingSegmentDataFormat => match text {
                                "Float" => {
                                    acquisition.segment_data_format = Some(DataFormat::Float)
                                }
                                _ => {
                                    panic!("Unknown segment data format: {}", text);
                                }
                            },
                            ParserState::ProcessingValueBytes => {
                                acquisition.value_bytes = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingMaxX => {
                                acquisition.max_x = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingMaxY => {
                                acquisition.max_y = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingPlumeStart => {
                                acquisition.plume_start = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingPlumeEnd => {
                                acquisition.plume_end = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingTemplate => {
                                acquisition.template = Some(text.to_owned())
                            }
                            ParserState::ProcessingProfilingType => match text {
                                "Global" => {
                                    acquisition.profiling_type = Some(ProfilingType::Global);
                                }
                                _ => {
                                    panic!("Unknown profiling type: {}", text);
                                }
                            },
                            ParserState::Processing => {}
                            _ => {
                                panic!(
                                    "Unknown sub state {:?} in state {:?} with event {:?}",
                                    self.sub_state, self.state, e
                                );
                            }
                        }

                        self.sub_state = ParserState::Processing
                    }

                    ParserState::ProcessingAcquisitionROI => {
                        let acquisition_roi = self.current_acquisition_roi.as_mut().unwrap();
                        let unprocessed_text = &e.unescaped().unwrap();
                        let text = std::str::from_utf8(unprocessed_text).unwrap();

                        match self.sub_state {
                            ParserState::ProcessingID => {
                                acquisition_roi.id = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingDescription => {
                                acquisition_roi.description = Some(text.to_string())
                            }
                            ParserState::ProcessingPanoramaID => {
                                acquisition_roi.panorama_id = Some(text.parse().unwrap())
                            }
                            ParserState::ProcessingROIType => match text {
                                "Acquisition" => {
                                    acquisition_roi.roi_type = Some(ROIType::Acquisition)
                                }
                                _ => {
                                    panic!("Unknown ROIType: {}", text);
                                }
                            },
                            ParserState::Processing => {}
                            _ => {
                                panic!(
                                    "Unknown sub state {:?} in state {:?} with event {:?}",
                                    self.sub_state, self.state, e
                                );
                            }
                        }

                        self.sub_state = ParserState::Processing
                    }

                    ParserState::ProcessingROIPoint => {
                        let roi_point = self.current_roi_point.as_mut().unwrap();
                        let unprocessed_text = &e.unescaped().unwrap();
                        let text = std::str::from_utf8(unprocessed_text).unwrap();

                        match self.sub_state {
                            ParserState::ProcessingID => roi_point.id = Some(text.parse().unwrap()),
                            ParserState::ProcessingAcquisitionROIID => {
                                roi_point.acquisition_roi_id = Some(text.parse().unwrap())
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
                            ParserState::Processing => {}
                            _ => {
                                panic!(
                                    "Unknown sub state {:?} in state {:?} with event {:?}",
                                    self.sub_state, self.state, e
                                );
                            }
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
