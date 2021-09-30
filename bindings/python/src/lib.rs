use pyo3::exceptions;
use pyo3::prelude::*;

use std::fs::File;

#[pyclass]
struct MCD {
    mcd: imc_rs::MCD<std::fs::File>,
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

        Ok(MCD { mcd: mcd })
    }

    pub fn num_slides(&self) -> PyResult<usize> {
        Ok(self.mcd.get_slide_ids().len())
    }

    pub fn slide_ids(&self) -> PyResult<Vec<u16>> {
        Ok(self.mcd.get_slide_ids())
    }

    pub fn panorama_ids(&self) -> PyResult<Vec<u16>> {
        let mut ids = Vec::new();

        for slide in self.mcd.get_slides() {
            ids.append(&mut slide.get_panorama_ids());
        }

        ids.sort();

        Ok(ids)
    }

    pub fn acquisition_ids(&self) -> PyResult<Vec<u16>> {
        let mut ids = Vec::new();

        for slide in self.mcd.get_slides() {
            for panorama in slide.get_panoramas() {
                ids.append(&mut panorama.get_acquisition_ids());
            }
        }

        ids.sort();

        Ok(ids)
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
