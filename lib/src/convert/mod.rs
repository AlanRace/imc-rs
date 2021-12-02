use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write},
    sync::{Arc, Mutex},
};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use rand::prelude::*;

use crate::{acquisition::DataLocation, MCD};

#[derive(Debug)]
#[allow(dead_code)]
struct AcquisitionOffset {
    id: u16,
    offsets: Vec<u64>,
    sizes: Vec<u64>,
}

enum Mode {
    None,
    Reading(BufReader<File>),
    Writing(BufWriter<File>),
}

struct TemporaryFile {
    name: String,

    mode: Mode,
}

impl TemporaryFile {
    fn new() -> Self {
        let mut rng = rand::thread_rng();
        let value: u64 = rng.gen();

        let filename = format!("{}.tmp", value);
        std::fs::File::create(&filename).unwrap();

        TemporaryFile {
            name: filename,

            mode: Mode::None,
        }
    }

    fn read_mode(&mut self) -> std::io::Result<()> {
        if let Mode::Writing(writer) = &mut self.mode {
            writer.flush()?;
        }

        self.mode = Mode::Reading(BufReader::new(File::open(&self.name)?));

        Ok(())
    }

    fn write_mode(&mut self) -> std::io::Result<()> {
        self.mode = Mode::Writing(BufWriter::new(
            std::fs::OpenOptions::new().write(true).open(&self.name)?,
        ));

        Ok(())
    }
}

impl Drop for TemporaryFile {
    fn drop(&mut self) {
        std::fs::remove_file(&self.name).unwrap();
    }
}

impl Seek for TemporaryFile {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        match &mut self.mode {
            Mode::None => Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "Can't seek unless we decide to read or write",
            )),
            Mode::Reading(reader) => reader.seek(pos),
            Mode::Writing(writer) => writer.seek(pos),
        }
    }
}

impl Read for TemporaryFile {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self.mode {
            Mode::None | Mode::Writing(_) => {
                self.read_mode()?;
            }
            _ => {}
        }

        match &mut self.mode {
            Mode::Reading(reader) => reader.read(buf),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "Can't read unless in read mode",
            )),
        }
    }
}

impl Write for TemporaryFile {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self.mode {
            Mode::None | Mode::Reading(_) => {
                self.write_mode()?;
            }
            _ => {}
        }

        match &mut self.mode {
            Mode::Writing(writer) => writer.write(buf),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "Can't write unless in write mode",
            )),
        }
    }

    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        match self.mode {
            Mode::None | Mode::Reading(_) => {
                self.write_mode()?;
            }
            _ => {}
        }

        match &mut self.mode {
            Mode::Writing(writer) => writer.write_all(buf),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "Can't write unless in write mode",
            )),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match &mut self.mode {
            Mode::Writing(writer) => writer.flush(),
            _ => Ok(()),
        }
    }
}

pub fn convert<T: Read + Seek>(mcd: &MCD<T>) -> std::io::Result<()> {
    let mut acquisition_offsets = HashMap::new();
    //println!("Opening {:?} for writing", mcd.dcm_file());
    let dcm_file = std::fs::File::create(mcd.dcm_file()).unwrap();
    let mut dcm_file = BufWriter::new(dcm_file);

    let mut num_acquisitions = 0;

    for slide in mcd.slides() {
        for panorama in slide.panoramas() {
            num_acquisitions += panorama.acquisitions().len();
        }
    }

    //println!("Writing {} acquisitions.", num_acquisitions);

    dcm_file.write_u8(num_acquisitions as u8)?;
    let index_location = dcm_file.seek(SeekFrom::Current(0)).unwrap();
    dcm_file.write_all(&vec![0; num_acquisitions * 11])?;

    let mut acquisition_index: Vec<(u16, u64, u8)> = Vec::new();

    for slide in mcd.slides() {
        for panorama in slide.panoramas() {
            for acquisition in panorama.acquisitions() {
                let mut files = Vec::with_capacity(acquisition.channels().len());

                for _channel in 0..acquisition.channels().len() {
                    files.push(TemporaryFile::new());
                }

                for spectrum in acquisition.spectra() {
                    for (channel_index, &value) in spectrum.iter().enumerate() {
                        files[channel_index].write_all(&value.to_le_bytes())?;
                    }
                }

                // Make sure all data is flushed to the temporary files
                for file in files.iter_mut() {
                    file.flush().unwrap();
                }

                let acquisition_index_location = dcm_file.seek(SeekFrom::Current(0)).unwrap();
                acquisition_index.push((
                    acquisition.id(),
                    acquisition_index_location,
                    acquisition.channels().len() as u8,
                ));

                // Write out empty data which we will overwrite with the indicies and sizes later
                dcm_file.write_all(&vec![0; acquisition.channels().len() * 16])?;

                let mut buf = vec![0; acquisition.num_spectra() * 4];
                let mut offsets = Vec::new();
                let mut sizes = Vec::new();

                for file in files.iter_mut() {
                    file.seek(SeekFrom::Start(0)).unwrap();
                    file.read_exact(&mut buf).unwrap();

                    let compressed = lz4_flex::compress(&buf);

                    let cur_location = dcm_file.seek(SeekFrom::Current(0)).unwrap();
                    dcm_file.write_all(&compressed).unwrap();
                    let new_location = dcm_file.seek(SeekFrom::Current(0)).unwrap();

                    offsets.push(cur_location);
                    sizes.push(new_location - cur_location);
                }

                let acquisition_end_location = dcm_file.seek(SeekFrom::Current(0)).unwrap();
                // Go back to where we wanted to write the index
                dcm_file
                    .seek(SeekFrom::Start(acquisition_index_location))
                    .unwrap();
                for (&offset, &size) in offsets.iter().zip(sizes.iter()) {
                    dcm_file.write_u64::<LittleEndian>(offset).unwrap();
                    dcm_file.write_u64::<LittleEndian>(size).unwrap();
                }

                // Reset the location to the end of the acquisition to continue writing
                dcm_file
                    .seek(SeekFrom::Start(acquisition_end_location))
                    .unwrap();

                acquisition_offsets.insert(
                    acquisition.id(),
                    AcquisitionOffset {
                        id: acquisition.id(),
                        offsets,
                        sizes,
                    },
                );

                //println!("{:?}", acquisition_offsets);
                //println!("{:?} done", acquisition.description());
            }
        }
    }

    // Go to location to write the index now we know where the data is stored
    dcm_file.seek(SeekFrom::Start(index_location)).unwrap();

    for &(acquisition_id, offset, num_channels) in &acquisition_index {
        dcm_file.write_u16::<LittleEndian>(acquisition_id).unwrap();
        dcm_file.write_u64::<LittleEndian>(offset).unwrap();
        dcm_file.write_u8(num_channels).unwrap();

        //  println!("Written: {}, {}", acquisition_id, offset);
    }

    dcm_file.flush().unwrap();

    Ok(())
}

pub fn open<T: Read + Seek>(mcd: &mut MCD<T>) -> std::io::Result<()> {
    //println!("Opening {:?} for reading", mcd.dcm_file());
    let dcm_file = std::fs::File::open(mcd.dcm_file()).unwrap();
    let dcm_file_arc = Arc::new(Mutex::new(BufReader::new(dcm_file)));
    let mut dcm_file = dcm_file_arc.lock().unwrap();

    let num_acquisitions = dcm_file.read_u8()?;
    let mut acquisition_offsets = HashMap::with_capacity(num_acquisitions as usize);

    for _i in 0..num_acquisitions {
        let id = dcm_file.read_u16::<LittleEndian>().expect("read id failed");
        let offset = dcm_file
            .read_u64::<LittleEndian>()
            .expect("read offset failed");
        let num_channels = dcm_file.read_u8().unwrap();

        acquisition_offsets.insert(id, (offset, num_channels));
    }

    //println!("Offset keys: {:?}", acquisition_offsets.keys());

    for slide in mcd.slides_mut().values_mut() {
        for panorama in slide.panoramas_mut().values_mut() {
            for acquisition in panorama.acquisitions_mut().values_mut() {
                let offset = acquisition_offsets.get(&acquisition.id());

                if let Some(&(offset, num_channels)) = offset {
                    dcm_file.seek(SeekFrom::Start(offset)).unwrap();
                    let mut offsets = vec![0; num_channels as usize];
                    let mut sizes = vec![0; num_channels as usize];

                    for (offset, size) in offsets.iter_mut().zip(sizes.iter_mut()) {
                        *offset = dcm_file.read_u64::<LittleEndian>().unwrap();
                        *size = dcm_file.read_u64::<LittleEndian>().unwrap();
                    }

                    //println!("Offsets: {:?}", offsets);
                    //println!("Size: {:?}", sizes);

                    acquisition.dcm_location = Some(DataLocation {
                        reader: dcm_file_arc.clone(),
                        offsets,
                        sizes,
                    });

                    /*println!(
                        "[{}] DCM Location: {:?}",
                        acquisition.description(),
                        acquisition.dcm_location
                    );*/
                }
            }
        }
    }

    Ok(())
}
