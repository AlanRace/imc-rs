mod parser;
mod xml_types;

use crate::{Acquisition, AcquisitionChannel, Slide, DataFormat, ImageFormat, Panorama, MCD};
use xml_types::{AcquisitionXML, AcquisitionChannelXML, AcquisitionROI, ROIPoint, SlideXML, ROIType, PanoramaXML};

pub use parser::MCDParser;
pub use parser::ParserState;