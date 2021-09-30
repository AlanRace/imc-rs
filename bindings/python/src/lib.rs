use pyo3::exceptions;
use pyo3::prelude::*;

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
