//! python bindings for imc-rs, a library for accessing imaging mass cytometry data.

use pyo3::exceptions;
use pyo3::prelude::*;

// For passing back images/data
use ndarray::Array;
use numpy::{IntoPyArray, PyArray3};

use std::fs::File;
use std::sync::Arc;

#[pyclass]
struct MCD {
    mcd: Arc<imc_rs::MCD<std::fs::File>>,
}



#[pymethods]
impl MCD {
    #[staticmethod]
    pub fn parse(filename: &str) -> PyResult<Self> {
        let file = match File::open(filename) {
            Ok(file) => file,
            Err(error) => return Err(PyErr::new::<exceptions::PyIOError, _>(error)),
        };

        let mcd = imc_rs::MCD::parse(file, filename);

        Ok(MCD { mcd: Arc::new(mcd) })
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
            None => Err(PyErr::new::<exceptions::PyValueError, _>(format!("No such slide with id {}", id)))
        }
    }

    pub fn panorama_ids(&self) -> PyResult<Vec<u16>> {
        let mut ids = Vec::new();

        for slide in self.mcd.slides() {
            ids.append(&mut slide.panorama_ids());
        }

        ids.sort();

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

        Err(PyErr::new::<exceptions::PyValueError, _>(format!("No such panorama with id {}", id)))
    }

    pub fn acquisition_ids(&self) -> PyResult<Vec<u16>> {
        let mut ids = Vec::new();

        for slide in self.mcd.slides() {
            for panorama in slide.panoramas() {
                ids.append(&mut panorama.acquisition_ids());
            }
        }

        ids.sort();

        Ok(ids)
    }
}


#[pyclass]
struct Slide {
    mcd: Arc<imc_rs::MCD<std::fs::File>>,

    id: u16,
}


#[pymethods]
impl Slide {
    pub fn id(&self) -> PyResult<u16> {
        Ok(self.id)
    }

    pub fn uid(&self) -> PyResult<String> {
        Ok(self.mcd.slide(self.id).expect("Slide ID was checked to exist during creation").uid().to_owned())
    }

    pub fn description(&self) -> PyResult<String> {
        Ok(self.mcd.slide(self.id).expect("Slide ID was checked to exist during creation").description().to_owned())
    }

    pub fn width_in_um(&self) -> PyResult<f64> {
        Ok(self.mcd.slide(self.id).expect("Slide ID was checked to exist during creation").width_in_um().to_owned())
    }

    pub fn height_in_um(&self) -> PyResult<f64> {
        Ok(self.mcd.slide(self.id).expect("Slide ID was checked to exist during creation").height_in_um().to_owned())
    }

    pub fn filename(&self) -> PyResult<String> {
        Ok(self.mcd.slide(self.id).expect("Slide ID was checked to exist during creation").filename().to_owned())
    }

    pub fn image_file(&self) -> PyResult<String> {
        Ok(self.mcd.slide(self.id).expect("Slide ID was checked to exist during creation").image_file().to_owned())
    }

    pub fn software_version(&self) -> PyResult<String> {
        Ok(self.mcd.slide(self.id).expect("Slide ID was checked to exist during creation").software_version().to_owned())
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
        self.mcd.slide(self.slide_id).expect("Should be valid slide id").panorama(self.id).expect("Should be valid panorama id")
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
}


#[pymodule]
fn pyimc(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<MCD>()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
