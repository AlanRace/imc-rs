use crate::acquisition::DataFormat;

use super::{AcquisitionChannel, ImageFormat};

#[derive(Debug)]
pub(crate) enum ROIType {
    Acquisition,
}

#[derive(Debug)]
pub(crate) struct AcquisitionROI {
    pub(crate) id: Option<u16>,
    pub(crate) panorama_id: Option<u16>,
    pub(crate) roi_type: Option<ROIType>,
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
        }
    }
}
