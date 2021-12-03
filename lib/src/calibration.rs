use crate::mcd::{
    CalibrationChannelXML, CalibrationFinalXML, CalibrationParamsXML, CalibrationXML,
};

pub struct CalibrationFinal {
    id: u16,
    acquisition_id: u16,
    time_stamp: String,
    optimal_detector_voltage_start: f64,
    optimal_detector_voltage_end: f64,
    optimal_detector_dual_coefficient_start: f64,
    optimal_detector_dual_coefficient_end: f64,
    optimal_helium: f64,
    transient_start: u32,
    transient_cross_talk_1: u32,
    transient_cross_talk_2: u32,
    reference_energy: f64,
    maximum_energy: f64,
}

impl From<CalibrationFinalXML> for CalibrationFinal {
    fn from(calibration_final: CalibrationFinalXML) -> Self {
        CalibrationFinal {
            id: calibration_final.id.unwrap(),
            acquisition_id: calibration_final.acquisition_id.unwrap(),
            time_stamp: calibration_final.time_stamp.unwrap(),
            optimal_detector_voltage_start: calibration_final
                .optimal_detector_voltage_start
                .unwrap(),
            optimal_detector_voltage_end: calibration_final.optimal_detector_voltage_end.unwrap(),
            optimal_detector_dual_coefficient_start: calibration_final
                .optimal_detector_dual_coefficient_start
                .unwrap(),
            optimal_detector_dual_coefficient_end: calibration_final
                .optimal_detector_dual_coefficient_end
                .unwrap(),
            optimal_helium: calibration_final.optimal_helium.unwrap(),
            transient_start: calibration_final.transient_start.unwrap(),
            transient_cross_talk_1: calibration_final.transient_cross_talk_1.unwrap(),
            transient_cross_talk_2: calibration_final.transient_cross_talk_2.unwrap(),
            reference_energy: calibration_final.reference_energy.unwrap(),
            maximum_energy: calibration_final.maximum_energy.unwrap(),
        }
    }
}

pub struct Calibration {
    id: u16,
    acquisition_id: u16,
    time_stamp: String,
}

impl From<CalibrationXML> for Calibration {
    fn from(calibration_final: CalibrationXML) -> Self {
        Calibration {
            id: calibration_final.id.unwrap(),
            acquisition_id: calibration_final.acquisition_id.unwrap(),
            time_stamp: calibration_final.time_stamp.unwrap(),
        }
    }
}

pub struct CalibrationParams {
    calibration_id: u16,
    optimal_detector_voltage: f64,
    optimal_detector_dual_coefficient: f64,
    optimal_makeup_gas: f64,
    optimal_current: f64,
    optimal_x: u32,
    optimal_y: u32,
    transient_start: u32,
    transient_cross_talk_1: f64,
    transient_cross_talk_2: f64,
    optimal_helium: f64,
}

impl From<CalibrationParamsXML> for CalibrationParams {
    fn from(calibration_params: CalibrationParamsXML) -> Self {
        CalibrationParams {
            calibration_id: calibration_params.calibration_id.unwrap(),
            optimal_detector_voltage: calibration_params.optimal_detector_voltage.unwrap(),
            optimal_detector_dual_coefficient: calibration_params
                .optimal_detector_dual_coefficient
                .unwrap(),
            optimal_makeup_gas: calibration_params.optimal_makeup_gas.unwrap(),
            optimal_current: calibration_params.optimal_current.unwrap(),
            optimal_x: calibration_params.optimal_x.unwrap(),
            optimal_y: calibration_params.optimal_y.unwrap(),
            transient_start: calibration_params.transient_start.unwrap(),
            transient_cross_talk_1: calibration_params.transient_cross_talk_1.unwrap(),
            transient_cross_talk_2: calibration_params.transient_cross_talk_2.unwrap(),
            optimal_helium: calibration_params.optimal_helium.unwrap(),
        }
    }
}

pub struct CalibrationChannel {
    calibration_id: u16,
    name: String,
    mean_duals: f64,
    id: u16,
}

impl From<CalibrationChannelXML> for CalibrationChannel {
    fn from(calibration_channel: CalibrationChannelXML) -> Self {
        CalibrationChannel {
            calibration_id: calibration_channel.calibration_id.unwrap(),
            name: calibration_channel.name.unwrap(),
            mean_duals: calibration_channel.mean_duals.unwrap(),
            id: calibration_channel.id.unwrap(),
        }
    }
}
