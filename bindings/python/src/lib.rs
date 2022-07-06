#![warn(missing_docs)]
#![warn(rustdoc::missing_doc_code_examples)]

//! python bindings for imc-rs, a library for accessing imaging mass cytometry data.

use imc_rs::error::MCDError;
use imc_rs::ChannelIdentifier;
use imc_rs::MCD;
use numpy::ndarray::Array;
use numpy::PyArray2;
use pyo3::exceptions;
use pyo3::exceptions::PyIOError;
use pyo3::prelude::*;

// For passing back images/data
use numpy::{IntoPyArray, PyArray3};

use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;

/// Mcd represents an .mcd file
#[pyclass]
struct Mcd {
    mcd: Arc<imc_rs::MCD<BufReader<File>>>,
}

struct PyMcdError(MCDError);

impl From<MCDError> for PyMcdError {
    fn from(error: MCDError) -> Self {
        PyMcdError(error)
    }
}

impl From<PyMcdError> for PyErr {
    fn from(error: PyMcdError) -> Self {
        PyIOError::new_err(error.0.to_string())
    }
}

#[pymethods]
impl Mcd {
    /// Parse an .mcd file, returning an object providing access to IMC data and accompanying metadata
    #[staticmethod]
    pub fn parse(filename: &str) -> PyResult<Self> {
        let file = match File::open(filename) {
            Ok(file) => file,
            Err(error) => return Err(PyErr::new::<exceptions::PyIOError, _>(error)),
        };

        let mcd = imc_rs::MCD::parse(BufReader::new(file), filename)?;

        Ok(Mcd { mcd: Arc::new(mcd) })
    }

    /// Parse an .mcd file, generating a temporary file for fast channel image access if one is not present, and
    /// returning an object providing access to IMC data and accompanying metadata
    #[staticmethod]
    pub fn parse_with_dcm(filename: &str) -> PyResult<Self> {
        let file = match File::open(filename) {
            Ok(file) => file,
            Err(error) => return Err(PyErr::new::<exceptions::PyIOError, _>(error)),
        };

        let mcd = match imc_rs::MCD::parse_with_dcm(BufReader::new(file), filename) {
            Ok(mcd) => mcd,
            Err(error) => return Err(PyMcdError::from(error).into()),
        };

        Ok(Mcd { mcd: Arc::new(mcd) })
    }

    /// Returns the number of slides in the .mcd data
    pub fn num_slides(&self) -> PyResult<usize> {
        Ok(self.mcd.slide_ids().len())
    }

    /// Returns a list of IDs for each slide in the .mcd data
    pub fn slide_ids(&self) -> PyResult<Vec<u16>> {
        Ok(self.mcd.slide_ids())
    }

    /// Returns the slide with the given ID, or None if none exists
    pub fn slide(&self, id: u16) -> Option<Slide> {
        self.mcd.slide(id).map(|_| Slide {
            mcd: self.mcd.clone(),
            id,
        })
    }

    /// Returns the XML data found within the .mcd file.
    pub fn xml(&self) -> PyResult<String> {
        match self.mcd.xml() {
            Ok(xml) => Ok(xml),
            Err(error) => Err(PyErr::new::<exceptions::PyIOError, _>(error)),
        }
    }

    /// Returns a sorted list of panorama IDs
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
                label: channel.label().to_string(),
            })
        }

        channels
    }
}

#[pyclass]
struct AcquisitionChannel {
    name: String,
    label: String,
}

#[pymethods]
impl AcquisitionChannel {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn label(&self) -> &str {
        &self.label
    }
}

#[pyclass]
struct Slide {
    mcd: Arc<MCD<BufReader<File>>>,

    id: u16,
}

impl Slide {
    fn get_slide(&self) -> &imc_rs::Slide<BufReader<File>> {
        self.mcd.slide(self.id).expect("Should be valid slide id")
    }
}

#[pymethods]
impl Slide {
    pub fn id(&self) -> PyResult<u16> {
        Ok(self.id)
    }

    /*pub fn uid(&self) -> PyResult<String> {
        Ok(self
            .mcd
            .slide(self.id)
            .expect("Slide ID was checked to exist during creation")
            .uid()
            .to_owned())
    }*/

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

        let image = slide.image().as_rgba8().unwrap();
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
    ) -> PyResult<&'py PyArray3<u8>> {
        let slide = self.get_slide();

        let overview_image = match channel {
            Some(channel) => {
                let identifier = ChannelIdentifier::Name(channel.name.clone());

                slide.create_overview_image(width.unwrap_or(7500), Some((&identifier, max_value)))
            }
            None => slide.create_overview_image(width.unwrap_or(7500), None),
        };

        let image = match overview_image {
            Ok(image) => image,
            Err(error) => {
                return Err(exceptions::PyIOError::new_err(error.to_string()));
            }
        };

        let width = image.width() as usize;
        let height = image.height() as usize;
        let raw_image = image.into_raw();

        //println!("image_raw = {}, array = ({}, {}, 3)", raw_image.len(), width, height);

        let array = Array::from_shape_vec((height, width, 4), raw_image).unwrap();
        Ok(array.into_pyarray(py))
    }
}

#[pyclass]
struct Panorama {
    mcd: Arc<MCD<BufReader<File>>>,

    id: u16,
    slide_id: u16,
}

impl Panorama {
    fn get_panorama(&self) -> &imc_rs::Panorama<BufReader<File>> {
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

        let image = panorama.image().unwrap().as_rgba8().unwrap();
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
    mcd: Arc<imc_rs::MCD<BufReader<File>>>,

    id: u16,
    panorama_id: u16,
    slide_id: u16,
}

impl Acquisition {
    fn get_acquisition(&self) -> &imc_rs::Acquisition<BufReader<File>> {
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

        let image = acquisition.before_ablation_image().as_rgba8().unwrap();
        let width = image.width() as usize;
        let height = image.height() as usize;
        let raw_image = image.into_raw();

        //println!("image_raw = {}, array = ({}, {}, 3)", raw_image.len(), width, height);

        let array = Array::from_shape_vec((height, width, 4), raw_image).unwrap();
        array.into_pyarray(py)
    }

    pub fn after_ablation_image<'py>(&self, py: Python<'py>) -> &'py PyArray3<u8> {
        let acquisition = self.get_acquisition();

        let image = acquisition.after_ablation_image().as_rgba8().unwrap();
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
                label: channel.label().to_string(),
            })
        }

        channels
    }

    pub fn channel_data<'py>(
        &self,
        channel: &'py AcquisitionChannel,
        py: Python<'py>,
    ) -> PyResult<&'py PyArray2<f32>> {
        let acquisition = self.get_acquisition();

        let identifier = ChannelIdentifier::Name(channel.name.clone());
        let channel_data = match acquisition.channel_image(&identifier, None) {
            Ok(channel_data) => channel_data,
            Err(error) => {
                return Err(exceptions::PyIOError::new_err(error.to_string()));
            }
        };

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
        Ok(array.into_pyarray(py))
    }
}

/// A Python module for reading and processing imaging mass cytometry data (stored in .mcd format).
///
/// # Quick start
///
/// >>> import pyimc
/// >>> data = pyimc.Mcd.parse_with_dcm("/path/to/data.mcd")
///
/// The above will generate a temporary file, which stores the acquisition data image-wise. This is
/// helpful when needing to access multiple images in a session as it significantly reduces the time
/// required to load a single image. The temporary file is stored in the same location as the .mcd file
/// and is only generated once. On the next call to `parse_with_dcm` no new file will be generated,
/// unless the existing temporary file is deleted. The temporary file is approximately 1/3 as large as the
/// .mcd file, but reduces the time required to read in a single channel image >200x.
///
/// To load the data without generating a temporary file (and reading the data spectrum-wise), use the
/// following command:
///
/// >>> import pyimc
/// >>> data = pyimc.Mcd.parse("/path/to/data.mcd")
///
///
/// Can generate a slide image helpful in whole slide imaging registration.
///
/// Also includes methods for reading in cell segmentation data produced by HALO (stored as .csv).
#[pymodule]
fn pyimc(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<Mcd>()?;

    Ok(())
}
