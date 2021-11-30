mod parser;
mod xml_types;

use crate::{Acquisition, AcquisitionChannel, ImageFormat, Panorama, MCD};
pub(crate) use xml_types::{
    AcquisitionChannelXML, AcquisitionROI, AcquisitionXML, PanoramaXML, ROIPoint, ROIType, SlideXML,
};

pub use parser::MCDParser;
pub use parser::ParserState;
