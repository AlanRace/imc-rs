use crate::{
    acquisition::{DataFormat, ProfilingType},
    panorama::PanoramaType,
};

use super::{AcquisitionChannel, ImageFormat};

#[derive(Debug)]
pub(crate) enum ROIType {
    Acquisition,
}

#[derive(Debug)]
pub(crate) struct AcquisitionROI {
    pub(crate) id: Option<u16>,
    // Description is only present in version 2 of the XSD
    pub(crate) description: Option<String>,
    pub(crate) panorama_id: Option<u16>,
    pub(crate) roi_type: Option<ROIType>,
}

impl AcquisitionROI {
    pub fn new() -> AcquisitionROI {
        AcquisitionROI {
            id: None,
            description: None,
            panorama_id: None,
            roi_type: None,
        }
    }
}

#[derive(Debug)]
pub(crate) struct ROIPoint {
    pub(crate) id: Option<u16>,
    pub(crate) acquisition_roi_id: Option<u16>,
    pub(crate) order_number: Option<i16>,
    pub(crate) slide_x_pos_um: Option<f64>,
    pub(crate) slide_y_pos_um: Option<f64>,
    pub(crate) panorama_pixel_x_pos: Option<i32>,
    pub(crate) panorama_pixel_y_pos: Option<i32>,
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

pub(crate) struct AcquisitionChannelXML {
    pub(crate) id: Option<u16>,
    pub(crate) channel_name: Option<String>,
    pub(crate) order_number: Option<i16>,
    pub(crate) acquisition_id: Option<u16>,
    pub(crate) channel_label: Option<String>,
}

impl AcquisitionChannelXML {
    pub fn new() -> AcquisitionChannelXML {
        AcquisitionChannelXML {
            id: None,
            channel_name: None,
            order_number: None,
            acquisition_id: None,
            channel_label: None,
        }
    }
}

impl From<AcquisitionChannelXML> for AcquisitionChannel {
    fn from(channel: AcquisitionChannelXML) -> Self {
        AcquisitionChannel::new(
            channel.id.expect("ID is required"),
            channel.acquisition_id.expect("AcquisitionID is required"),
            channel.order_number.expect("OrderNumber is required"),
            &channel.channel_name.expect("ChannelName is required"),
            &channel.channel_label.expect("ChannelLabel is required"),
        )
    }
}

#[derive(Debug)]
pub(crate) struct AcquisitionXML {
    pub(crate) id: Option<u16>,
    pub(crate) description: Option<String>,
    pub(crate) ablation_power: Option<f64>,
    pub(crate) ablation_distance_between_shots_x: Option<f64>,
    pub(crate) ablation_distance_between_shots_y: Option<f64>,
    pub(crate) ablation_frequency: Option<f64>,
    pub(crate) acquisition_roi_id: Option<i16>,
    pub(crate) order_number: Option<i16>,
    pub(crate) signal_type: Option<String>,
    pub(crate) dual_count_start: Option<String>,
    pub(crate) data_start_offset: Option<i64>,
    pub(crate) data_end_offset: Option<i64>,
    pub(crate) start_timestamp: Option<String>,
    pub(crate) end_timestamp: Option<String>,
    pub(crate) after_ablation_image_start_offset: Option<i64>,
    pub(crate) after_ablation_image_end_offset: Option<i64>,
    pub(crate) before_ablation_image_start_offset: Option<i64>,
    pub(crate) before_ablation_image_end_offset: Option<i64>,
    pub(crate) roi_start_x_pos_um: Option<f64>,
    pub(crate) roi_start_y_pos_um: Option<f64>,
    pub(crate) roi_end_x_pos_um: Option<f64>,
    pub(crate) roi_end_y_pos_um: Option<f64>,
    pub(crate) movement_type: Option<String>,
    pub(crate) segment_data_format: Option<DataFormat>,
    pub(crate) value_bytes: Option<u8>,
    pub(crate) max_x: Option<i32>,
    pub(crate) max_y: Option<i32>,
    pub(crate) plume_start: Option<i32>,
    pub(crate) plume_end: Option<i32>,
    pub(crate) template: Option<String>,
    pub(crate) profiling_type: Option<ProfilingType>,
}

impl AcquisitionXML {
    pub fn new() -> AcquisitionXML {
        AcquisitionXML {
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
            profiling_type: None,
        }
    }
}

pub(crate) struct SlideXML {
    pub(crate) id: Option<u16>,
    pub(crate) uid: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) filename: Option<String>,
    pub(crate) slide_type: Option<String>,
    pub(crate) width_um: Option<f64>,
    pub(crate) height_um: Option<f64>,

    pub(crate) image_start_offset: Option<i64>,
    pub(crate) image_end_offset: Option<i64>,
    pub(crate) image_file: Option<String>,

    pub(crate) energy_db: Option<u32>,
    pub(crate) frequency: Option<u32>,
    pub(crate) fmark_slide_length: Option<u64>,
    pub(crate) fmark_slide_thickness: Option<u64>,
    pub(crate) name: Option<String>,

    pub(crate) sw_version: Option<String>,
}

impl SlideXML {
    pub fn new() -> SlideXML {
        SlideXML {
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
            energy_db: None,
            frequency: None,
            fmark_slide_length: None,
            fmark_slide_thickness: None,
            name: None,
            sw_version: None,
        }
    }
}

#[derive(Debug)]
pub(crate) struct PanoramaXML {
    pub(crate) id: Option<u16>,
    pub(crate) slide_id: Option<u16>,
    pub(crate) description: Option<String>,
    pub(crate) slide_x1_pos_um: Option<f64>,
    pub(crate) slide_y1_pos_um: Option<f64>,
    pub(crate) slide_x2_pos_um: Option<f64>,
    pub(crate) slide_y2_pos_um: Option<f64>,
    pub(crate) slide_x3_pos_um: Option<f64>,
    pub(crate) slide_y3_pos_um: Option<f64>,
    pub(crate) slide_x4_pos_um: Option<f64>,
    pub(crate) slide_y4_pos_um: Option<f64>,

    pub(crate) image_start_offset: Option<i64>,
    pub(crate) image_end_offset: Option<i64>,
    pub(crate) pixel_width: Option<i64>,
    pub(crate) pixel_height: Option<i64>,
    pub(crate) image_format: Option<ImageFormat>,
    pub(crate) pixel_scale_coef: Option<f64>,

    pub(crate) panorama_type: Option<PanoramaType>,
    pub(crate) is_locked: Option<bool>,
    pub(crate) rotation_angle: Option<u16>,
}

impl PanoramaXML {
    pub fn new() -> Self {
        PanoramaXML {
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
            panorama_type: None,
            is_locked: None,
            rotation_angle: None,
        }
    }
}

pub(crate) struct CalibrationFinalXML {
    pub(crate) id: Option<u16>,
    pub(crate) acquisition_id: Option<u16>,
    pub(crate) time_stamp: Option<String>,
    pub(crate) optimal_detector_voltage_start: Option<f64>,
    pub(crate) optimal_detector_voltage_end: Option<f64>,
    pub(crate) optimal_detector_dual_coefficient_start: Option<f64>,
    pub(crate) optimal_detector_dual_coefficient_end: Option<f64>,
    pub(crate) optimal_helium: Option<f64>,
    pub(crate) transient_start: Option<u32>,
    pub(crate) transient_cross_talk_1: Option<u32>,
    pub(crate) transient_cross_talk_2: Option<u32>,
    pub(crate) reference_energy: Option<f64>,
    pub(crate) maximum_energy: Option<f64>,
}

impl CalibrationFinalXML {
    pub fn new() -> Self {
        CalibrationFinalXML {
            id: None,
            acquisition_id: None,
            time_stamp: None,
            optimal_detector_voltage_start: None,
            optimal_detector_voltage_end: None,
            optimal_detector_dual_coefficient_start: None,
            optimal_detector_dual_coefficient_end: None,
            optimal_helium: None,
            transient_start: None,
            transient_cross_talk_1: None,
            transient_cross_talk_2: None,
            reference_energy: None,
            maximum_energy: None,
        }
    }
}

pub(crate) struct CalibrationXML {
    pub(crate) id: Option<u16>,
    pub(crate) acquisition_id: Option<u16>,
    pub(crate) time_stamp: Option<String>,
}

impl CalibrationXML {
    pub fn new() -> Self {
        CalibrationXML {
            id: None,
            acquisition_id: None,
            time_stamp: None,
        }
    }
}

pub struct CalibrationParamsXML {
    pub(crate) calibration_id: Option<u16>,
    pub(crate) optimal_detector_voltage: Option<f64>,
    pub(crate) optimal_detector_dual_coefficient: Option<f64>,
    pub(crate) optimal_makeup_gas: Option<f64>,
    pub(crate) optimal_current: Option<f64>,
    pub(crate) optimal_x: Option<u32>,
    pub(crate) optimal_y: Option<u32>,
    pub(crate) transient_start: Option<u32>,
    pub(crate) transient_cross_talk_1: Option<f64>,
    pub(crate) transient_cross_talk_2: Option<f64>,
    pub(crate) optimal_helium: Option<f64>,
}

impl CalibrationParamsXML {
    pub fn new() -> Self {
        CalibrationParamsXML {
            calibration_id: None,
            optimal_detector_voltage: None,
            optimal_detector_dual_coefficient: None,
            optimal_makeup_gas: None,
            optimal_current: None,
            optimal_x: None,
            optimal_y: None,
            transient_start: None,
            transient_cross_talk_1: None,
            transient_cross_talk_2: None,
            optimal_helium: None,
        }
    }
}

impl Default for CalibrationParamsXML {
    fn default() -> Self {
        Self::new()
    }
}

pub struct CalibrationChannelXML {
    pub(crate) calibration_id: Option<u16>,
    pub(crate) name: Option<String>,
    pub(crate) mean_duals: Option<f64>,
    pub(crate) id: Option<u16>,
}

impl Default for CalibrationChannelXML {
    fn default() -> Self {
        Self::new()
    }
}

impl CalibrationChannelXML {
    pub fn new() -> Self {
        CalibrationChannelXML {
            calibration_id: None,
            name: None,
            mean_duals: None,
            id: None,
        }
    }
}

pub struct SlideFiducialMarksXML {
    pub(crate) id: Option<u16>,
    pub(crate) slide_id: Option<u16>,
    pub(crate) coordinate_x: Option<u32>,
    pub(crate) coordinate_y: Option<u32>,
}

impl SlideFiducialMarksXML {
    pub fn new() -> Self {
        SlideFiducialMarksXML {
            id: None,
            slide_id: None,
            coordinate_x: None,
            coordinate_y: None,
        }
    }
}

impl Default for SlideFiducialMarksXML {
    fn default() -> Self {
        Self::new()
    }
}

pub struct SlideProfileXML {
    pub(crate) id: Option<u16>,
    pub(crate) slide_id: Option<u16>,
    pub(crate) coordinate_x: Option<u32>,
    pub(crate) coordinate_y: Option<u32>,
}

impl SlideProfileXML {
    pub fn new() -> Self {
        SlideProfileXML {
            id: None,
            slide_id: None,
            coordinate_x: None,
            coordinate_y: None,
        }
    }
}

impl Default for SlideProfileXML {
    fn default() -> Self {
        Self::new()
    }
}
