//! python bindings for imc-rs, a library for accessing imaging mass cytometry data.

use imc_rs::ChannelIdentifier;
use numpy::PyArray2;
use pyo3::exceptions;
use pyo3::prelude::*;

// For passing back images/data
use ndarray::Array;
use numpy::{IntoPyArray, PyArray3};

use std::fs::File;
use std::sync::Arc;

#[pyclass]
struct Mcd {
    mcd: Arc<imc_rs::MCD<std::fs::File>>,
}

#[pymethods]
impl Mcd {
    #[staticmethod]
    pub fn parse(filename: &str) -> PyResult<Self> {
        let file = match File::open(filename) {
            Ok(file) => file,
            Err(error) => return Err(PyErr::new::<exceptions::PyIOError, _>(error)),
        };

        let mcd = imc_rs::MCD::parse(file, filename);

        Ok(Mcd { mcd: Arc::new(mcd) })
    }

    #[staticmethod]
    pub fn parse_with_dcm(filename: &str) -> PyResult<Self> {
        let file = match File::open(filename) {
            Ok(file) => file,
            Err(error) => return Err(PyErr::new::<exceptions::PyIOError, _>(error)),
        };

        let mcd = imc_rs::MCD::parse_with_dcm(file, filename);

        Ok(Mcd { mcd: Arc::new(mcd) })
    }

    pub fn num_slides(&self) -> PyResult<usize> {
        Ok(self.mcd.slide_ids().len())
    }

    pub fn slide_ids(&self) -> PyResult<Vec<u16>> {
        Ok(self.mcd.slide_ids())
    }

    pub fn slide(&self, id: u16) -> PyResult<Slide> {
        match self.mcd.slide(id) {
            Some(_) => Ok(Slide {
                mcd: self.mcd.clone(),
                id,
            }),
            None => Err(PyErr::new::<exceptions::PyValueError, _>(format!(
                "No such slide with id {}",
                id
            ))),
        }
    }

    pub fn panorama_ids(&self) -> PyResult<Vec<u16>> {
        let mut ids = Vec::new();

        for slide in self.mcd.slides() {
            ids.append(&mut slide.panorama_ids());
        }

        ids.sort_unstable();

        Ok(ids)
    }

    pub fn panorama(&self, id: u16) -> PyResult<Panorama> {
        for slide in self.mcd.slides() {
            for panorama in slide.panoramas() {
                if panorama.id() == id {
                    return Ok(Panorama {
                        mcd: self.mcd.clone(),
                        id,
                        slide_id: slide.id(),
                    });
                }
            }
        }

        Err(PyErr::new::<exceptions::PyValueError, _>(format!(
            "No such panorama with id {}",
            id
        )))
    }

    pub fn acquisition_ids(&self) -> PyResult<Vec<u16>> {
        let mut ids = Vec::new();

        for slide in self.mcd.slides() {
            for panorama in slide.panoramas() {
                ids.append(&mut panorama.acquisition_ids());
            }
        }

        ids.sort_unstable();

        Ok(ids)
    }

    pub fn acquisition(&self, id: u16) -> PyResult<Acquisition> {
        for slide in self.mcd.slides() {
            for panorama in slide.panoramas() {
                for acquisition in panorama.acquisitions() {
                    if acquisition.id() == id {
                        return Ok(Acquisition {
                            mcd: self.mcd.clone(),
                            id,
                            panorama_id: panorama.id(),
                            slide_id: slide.id(),
                        });
                    }
                }
            }
        }

        Err(PyErr::new::<exceptions::PyValueError, _>(format!(
            "No such acquisition with id {}",
            id
        )))
    }

    pub fn channels(&self) -> Vec<AcquisitionChannel> {
        let mut channels = Vec::new();

        for channel in self.mcd.channels() {
            channels.push(AcquisitionChannel {
                name: channel.name().to_string(),
            })
        }

        channels
    }
}

#[pyclass]
struct AcquisitionChannel {
    name: String,
}

#[pymethods]
impl AcquisitionChannel {
    pub fn name(&self) -> &str {
        &self.name
    }
}

#[pyclass]
struct Slide {
    mcd: Arc<imc_rs::MCD<std::fs::File>>,

    id: u16,
}

impl Slide {
    fn get_slide(&self) -> &imc_rs::Slide<std::fs::File> {
        self.mcd.slide(self.id).expect("Should be valid slide id")
    }
}

#[pymethods]
impl Slide {
    pub fn id(&self) -> PyResult<u16> {
        Ok(self.id)
    }

    pub fn uid(&self) -> PyResult<String> {
        Ok(self
            .mcd
            .slide(self.id)
            .expect("Slide ID was checked to exist during creation")
            .uid()
            .to_owned())
    }

    pub fn description(&self) -> PyResult<String> {
        Ok(self
            .mcd
            .slide(self.id)
            .expect("Slide ID was checked to exist during creation")
            .description()
            .to_owned())
    }

    pub fn width_in_um(&self) -> PyResult<f64> {
        Ok(self
            .mcd
            .slide(self.id)
            .expect("Slide ID was checked to exist during creation")
            .width_in_um()
            .to_owned())
    }

    pub fn height_in_um(&self) -> PyResult<f64> {
        Ok(self
            .mcd
            .slide(self.id)
            .expect("Slide ID was checked to exist during creation")
            .height_in_um()
            .to_owned())
    }

    pub fn filename(&self) -> PyResult<String> {
        Ok(self
            .mcd
            .slide(self.id)
            .expect("Slide ID was checked to exist during creation")
            .filename()
            .to_owned())
    }

    pub fn image_file(&self) -> PyResult<String> {
        Ok(self
            .mcd
            .slide(self.id)
            .expect("Slide ID was checked to exist during creation")
            .image_file()
            .to_owned())
    }

    pub fn software_version(&self) -> PyResult<String> {
        Ok(self
            .mcd
            .slide(self.id)
            .expect("Slide ID was checked to exist during creation")
            .software_version()
            .to_owned())
    }

    pub fn image<'py>(&self, py: Python<'py>) -> &'py PyArray3<u8> {
        let slide = self.get_slide();

        let image = slide.image();
        let width = image.width() as usize;
        let height = image.height() as usize;
        let raw_image = image.into_raw();

        //println!("image_raw = {}, array = ({}, {}, 3)", raw_image.len(), width, height);

        let array = Array::from_shape_vec((height, width, 3), raw_image).unwrap();
        array.into_pyarray(py)
    }

    pub fn overview_image<'py>(
        &self,
        width: Option<u32>,
        channel: Option<&'py AcquisitionChannel>,
        max_value: Option<f32>,
        py: Python<'py>,
    ) -> &'py PyArray3<u8> {
        let slide = self.get_slide();

        let image = match channel {
            Some(channel) => {
                let identifier = ChannelIdentifier::Name(channel.name.clone());

                slide.create_overview_image(width.unwrap_or(7500), Some((&identifier, max_value)))
            }
            None => slide.create_overview_image(width.unwrap_or(7500), None),
        };
        let width = image.width() as usize;
        let height = image.height() as usize;
        let raw_image = image.into_raw();

        //println!("image_raw = {}, array = ({}, {}, 3)", raw_image.len(), width, height);

        let array = Array::from_shape_vec((height, width, 4), raw_image).unwrap();
        array.into_pyarray(py)
    }
}

#[pyclass]
struct Panorama {
    mcd: Arc<imc_rs::MCD<std::fs::File>>,

    id: u16,
    slide_id: u16,
}

impl Panorama {
    fn get_panorama(&self) -> &imc_rs::Panorama<std::fs::File> {
        self.mcd
            .slide(self.slide_id)
            .expect("Should be valid slide id")
            .panorama(self.id)
            .expect("Should be valid panorama id")
    }
}

#[pymethods]
impl Panorama {
    pub fn id(&self) -> PyResult<u16> {
        Ok(self.id)
    }

    pub fn slide_id(&self) -> PyResult<u16> {
        Ok(self.slide_id)
    }

    pub fn image<'py>(&self, py: Python<'py>) -> &'py PyArray3<u8> {
        let panorama = self.get_panorama();

        let image = panorama.image();
        let width = image.width() as usize;
        let height = image.height() as usize;
        let raw_image = image.into_raw();

        //println!("image_raw = {}, array = ({}, {}, 3)", raw_image.len(), width, height);

        let array = Array::from_shape_vec((height, width, 4), raw_image).unwrap();
        array.into_pyarray(py)
    }

    pub fn acquisition_ids(&self) -> PyResult<Vec<u16>> {
        Ok(self.get_panorama().acquisition_ids())
    }
}

#[pyclass]
struct Acquisition {
    mcd: Arc<imc_rs::MCD<std::fs::File>>,

    id: u16,
    panorama_id: u16,
    slide_id: u16,
}

impl Acquisition {
    fn get_acquisition(&self) -> &imc_rs::Acquisition<std::fs::File> {
        self.mcd
            .slide(self.slide_id)
            .expect("Should be valid slide id")
            .panorama(self.panorama_id)
            .expect("Should be valid panorama id")
            .acquisition(self.id)
            .expect("Should be valid acquisition id")
    }
}

#[pymethods]
impl Acquisition {
    pub fn id(&self) -> PyResult<u16> {
        Ok(self.id)
    }

    pub fn panorama_id(&self) -> PyResult<u16> {
        Ok(self.panorama_id)
    }

    pub fn slide_id(&self) -> PyResult<u16> {
        Ok(self.slide_id)
    }

    pub fn before_ablation_image<'py>(&self, py: Python<'py>) -> &'py PyArray3<u8> {
        let acquisition = self.get_acquisition();

        let image = acquisition.before_ablation_image();
        let width = image.width() as usize;
        let height = image.height() as usize;
        let raw_image = image.into_raw();

        //println!("image_raw = {}, array = ({}, {}, 3)", raw_image.len(), width, height);

        let array = Array::from_shape_vec((height, width, 4), raw_image).unwrap();
        array.into_pyarray(py)
    }

    pub fn after_ablation_image<'py>(&self, py: Python<'py>) -> &'py PyArray3<u8> {
        let acquisition = self.get_acquisition();

        let image = acquisition.after_ablation_image();
        let width = image.width() as usize;
        let height = image.height() as usize;
        let raw_image = image.into_raw();

        //println!("image_raw = {}, array = ({}, {}, 3)", raw_image.len(), width, height);

        let array = Array::from_shape_vec((height, width, 4), raw_image).unwrap();
        array.into_pyarray(py)
    }

    pub fn channels(&self) -> Vec<AcquisitionChannel> {
        let acquisition = self.get_acquisition();

        let mut channels = Vec::new();

        for channel in acquisition.channels() {
            channels.push(AcquisitionChannel {
                name: channel.name().to_string(),
            })
        }

        channels
    }

    pub fn channel_data<'py>(
        &self,
        channel: &'py AcquisitionChannel,
        py: Python<'py>,
    ) -> &'py PyArray2<f32> {
        let acquisition = self.get_acquisition();

        let identifier = ChannelIdentifier::Name(channel.name.clone());
        let channel_data = acquisition.channel_data(&identifier);

        let data = match !channel_data.is_complete() {
            true => channel_data.intensities().to_vec(),
            false => {
                let mut data =
                    vec![0.0; acquisition.height() as usize * acquisition.width() as usize];

                let intensities = channel_data.intensities();

                for (i, &intensity) in intensities.iter().enumerate() {
                    data[i] = intensity;
                }

                data
            }
        };

        let array = Array::from_shape_vec(
            (acquisition.height() as usize, acquisition.width() as usize),
            data,
        )
        .unwrap();
        array.into_pyarray(py)
    }
}

#[pymodule]
fn pyimc(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<Mcd>()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
