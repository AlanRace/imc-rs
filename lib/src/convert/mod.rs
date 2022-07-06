use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader, BufWriter, Cursor, Read, Seek, SeekFrom, Write},
    sync::{Arc, Mutex},
};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use crate::{error::MCDError, Acquisition, Region, MCD};

#[derive(Debug)]
#[allow(dead_code)]
struct AcquisitionOffset {
    id: u16,
    offsets: Vec<u64>,
    sizes: Vec<u64>,
}

// Format
// -----------
// chunk size (u8)
// number of acquisitions (u8)
// offsets for each acquisition ((u16, u64, u8))

#[derive(Debug)]
struct AcquisitionDetails {
    width: u32,
    height: u32,
    num_spectra: u32,

    chunk_size: u32,

    chunks: Vec<PixelChunk>,
}

impl AcquisitionDetails {
    fn from<T: BufRead + Seek>(acquisition: &Acquisition<T>, chunk_size: u32) -> Self {
        AcquisitionDetails {
            width: acquisition.width() as u32,
            height: acquisition.height() as u32,
            num_spectra: acquisition.num_spectra() as u32,
            chunk_size,
            chunks: Vec::new(),
        }
    }

    fn acquired_width(&self) -> u32 {
        if self.width <= self.num_spectra {
            self.width
        } else {
            self.num_spectra
        }
    }

    fn acquired_height(&self) -> u32 {
        (self.num_spectra / self.width) + 1
    }

    fn num_chunks_x(&self) -> u32 {
        (self.acquired_width() / self.chunk_size) + 1
    }

    fn num_chunks_y(&self) -> u32 {
        (self.acquired_height() / self.chunk_size) + 1
    }
}

#[derive(Debug)]
struct ChannelChunk {
    num_intensities: u64,
    offset: u64,
    length: u64,
}

#[derive(Debug)]
struct PixelChunk {
    channels: Vec<ChannelChunk>,
}

impl PixelChunk {
    fn new() -> Self {
        PixelChunk {
            channels: Vec::new(),
        }
    }
}

pub fn convert<T: BufRead + Seek>(mcd: &MCD<T>) -> Result<(), MCDError> {
    //let mut acquisition_offsets = HashMap::new();
    //println!("Opening {:?} for writing", mcd.dcm_file());
    let dcm_file = std::fs::File::create(mcd.dcm_file())?;
    let mut dcm_file = BufWriter::new(dcm_file);

    let mut num_acquisitions = 0;

    for slide in mcd.slides() {
        for panorama in slide.panoramas() {
            num_acquisitions += panorama.acquisitions().len();
        }
    }

    //println!("Writing {} acquisitions.", num_acquisitions);

    let chunk_size = 128;

    //dcm_file.write_u8(chunk_size as u8)?;

    dcm_file.write_u8(num_acquisitions as u8)?;
    let index_location = dcm_file.seek(SeekFrom::Current(0))?;
    dcm_file.write_all(&vec![0; num_acquisitions * 10])?;

    let mut acquisition_index: Vec<(u16, u64)> = Vec::new();

    for slide in mcd.slides() {
        for panorama in slide.panoramas() {
            for acquisition in panorama.acquisitions() {
                let mut acq_details = AcquisitionDetails::from(acquisition, chunk_size);

                println!(
                    "[{}] Total # chunks: ({}, {})",
                    acquisition.description(),
                    acq_details.num_chunks_x(),
                    acq_details.num_chunks_y()
                );

                for y_chunk in 0..acq_details.num_chunks_y() {
                    for x_chunk in 0..acq_details.num_chunks_x() {
                        let x_start = x_chunk * chunk_size;
                        let x_stop = (x_start + chunk_size).min(acq_details.acquired_width());

                        let y_start = y_chunk * chunk_size;
                        let y_stop = (y_start + chunk_size).min(acq_details.acquired_height());

                        let chunk_width = x_stop - x_start;
                        let chunk_height = y_stop - y_start;

                        //println!("[{}] ({}, {})", acquisition.description(), x_chunk, y_chunk);

                        let mut channel_chunks = Vec::with_capacity(acquisition.channels().len());

                        for _ in 0..acquisition.channels().len() {
                            channel_chunks.push(Vec::with_capacity(
                                chunk_width as usize * chunk_height as usize,
                            ));
                        }

                        for y in y_start..y_stop {
                            for x in x_start..x_stop {
                                let spectrum = match acquisition.spectrum(x as usize, y as usize) {
                                    Ok(spectrum) => spectrum,
                                    Err(MCDError::InvalidIndex {
                                        index: _,
                                        num_spectra: _,
                                    }) => {
                                        break;
                                    }
                                    Err(error) => {
                                        return Err(error);
                                    }
                                };

                                for (index, intensity) in spectrum.iter().enumerate() {
                                    channel_chunks[index].push(*intensity);
                                }
                            }
                        }

                        let mut pixel_chunk = PixelChunk::new();

                        for channel_chunk in channel_chunks {
                            let num_intensities = channel_chunk.len();

                            let mut buf: Vec<u8> = Vec::with_capacity(channel_chunk.len() * 4);

                            for intensity in channel_chunk {
                                buf.write_f32::<LittleEndian>(intensity)?;
                            }

                            let compressed = lz4_flex::compress(&buf);

                            let cur_location = dcm_file.seek(SeekFrom::Current(0))?;
                            dcm_file.write_all(&compressed)?;
                            let new_location = dcm_file.seek(SeekFrom::Current(0))?;

                            pixel_chunk.channels.push(ChannelChunk {
                                num_intensities: num_intensities as u64,
                                offset: cur_location,
                                length: new_location - cur_location,
                            });
                        }

                        acq_details.chunks.push(pixel_chunk);
                    }
                }

                let acquisition_index_location = dcm_file.seek(SeekFrom::Current(0)).unwrap();
                acquisition_index.push((acquisition.id(), acquisition_index_location));

                dcm_file.write_acquisition_details(&acq_details)?;
            }
        }
    }

    dcm_file.flush()?;

    // Go to location to write the index now we know where the data is stored
    dcm_file.seek(SeekFrom::Start(index_location))?;

    for &(acquisition_id, offset) in &acquisition_index {
        dcm_file.write_u16::<LittleEndian>(acquisition_id)?;
        dcm_file.write_u64::<LittleEndian>(offset)?;

        //  println!("Written: {}, {}", acquisition_id, offset);
    }

    dcm_file.flush()?;

    Ok(())
}

trait ReadDCM {
    fn read_acquisition_details(&mut self) -> std::io::Result<AcquisitionDetails>;
    fn read_pixel_chunk(&mut self) -> std::io::Result<PixelChunk>;
    fn read_channel_chunk(&mut self) -> std::io::Result<ChannelChunk>;
}

trait WriteDCM {
    fn write_acquisition_details(&mut self, details: &AcquisitionDetails) -> std::io::Result<()>;
    fn write_pixel_chunk(&mut self, chunk: &PixelChunk) -> std::io::Result<()>;
    fn write_channel_chunk(&mut self, chunk: &ChannelChunk) -> std::io::Result<()>;
}

impl<T: ReadBytesExt> ReadDCM for T {
    fn read_acquisition_details(&mut self) -> std::io::Result<AcquisitionDetails> {
        let width = self.read_u32::<LittleEndian>()?;
        let height = self.read_u32::<LittleEndian>()?;
        let num_spectra = self.read_u32::<LittleEndian>()?;
        let chunk_size = self.read_u32::<LittleEndian>()?;
        let num_chunks = self.read_u64::<LittleEndian>()?;

        let mut chunks = Vec::with_capacity(num_chunks as usize);

        for _ in 0..num_chunks {
            chunks.push(self.read_pixel_chunk()?);
        }

        Ok(AcquisitionDetails {
            width,
            height,
            num_spectra,
            chunk_size,
            chunks,
        })
    }

    fn read_pixel_chunk(&mut self) -> std::io::Result<PixelChunk> {
        let num_channels = self.read_u64::<LittleEndian>()?;

        let mut channels = Vec::with_capacity(num_channels as usize);

        for _ in 0..num_channels {
            channels.push(self.read_channel_chunk()?);
        }

        Ok(PixelChunk { channels })
    }

    fn read_channel_chunk(&mut self) -> std::io::Result<ChannelChunk> {
        let num_intensities = self.read_u64::<LittleEndian>()?;
        let offset = self.read_u64::<LittleEndian>()?;
        let length = self.read_u64::<LittleEndian>()?;

        Ok(ChannelChunk {
            num_intensities,
            offset,
            length,
        })
    }
}

impl<T: WriteBytesExt> WriteDCM for T {
    fn write_acquisition_details(&mut self, details: &AcquisitionDetails) -> std::io::Result<()> {
        self.write_u32::<LittleEndian>(details.width)?;
        self.write_u32::<LittleEndian>(details.height)?;
        self.write_u32::<LittleEndian>(details.num_spectra)?;
        self.write_u32::<LittleEndian>(details.chunk_size)?;
        self.write_u64::<LittleEndian>(details.chunks.len() as u64)?;

        for chunk in &details.chunks {
            self.write_pixel_chunk(chunk)?;
        }

        Ok(())
    }

    fn write_pixel_chunk(&mut self, chunk: &PixelChunk) -> std::io::Result<()> {
        self.write_u64::<LittleEndian>(chunk.channels.len() as u64)?;

        for channel in &chunk.channels {
            self.write_channel_chunk(channel)?;
        }

        Ok(())
    }

    fn write_channel_chunk(&mut self, chunk: &ChannelChunk) -> std::io::Result<()> {
        self.write_u64::<LittleEndian>(chunk.num_intensities)?;
        self.write_u64::<LittleEndian>(chunk.offset)?;
        self.write_u64::<LittleEndian>(chunk.length)?;

        Ok(())
    }
}

pub fn open<T: BufRead + Seek>(mcd: &mut MCD<T>) -> Result<(), MCDError> {
    //println!("Opening {:?} for reading", mcd.dcm_file());
    let dcm_file = std::fs::File::open(mcd.dcm_file())?;
    let dcm_file_arc = Arc::new(Mutex::new(BufReader::new(dcm_file)));
    let mut dcm_file = dcm_file_arc.lock().unwrap();

    let num_acquisitions = dcm_file.read_u8()?;

    let mut acquisition_offsets = HashMap::with_capacity(num_acquisitions as usize);

    for _i in 0..num_acquisitions {
        let id = dcm_file.read_u16::<LittleEndian>().expect("read id failed");
        let offset = dcm_file
            .read_u64::<LittleEndian>()
            .expect("read offset failed");
        //let num_channels = dcm_file.read_u8().unwrap();

        acquisition_offsets.insert(id, offset);
    }

    // println!("Offsets: {:?}", acquisition_offsets);

    for slide in mcd.slides_mut().values_mut() {
        for panorama in slide.panoramas_mut().values_mut() {
            for acquisition in panorama.acquisitions_mut().values_mut() {
                let offset = acquisition_offsets.get(&acquisition.id());

                if let Some(&offset) = offset {
                    dcm_file.seek(SeekFrom::Start(offset))?;

                    let acquisition_details = dcm_file.read_acquisition_details()?;

                    acquisition.dcm_location = Some(DCMLocation {
                        reader: dcm_file_arc.clone(),
                        details: acquisition_details,
                    });
                }
            }
        }
    }

    Ok(())
}

#[derive(Debug)]
pub struct DCMLocation {
    reader: Arc<Mutex<BufReader<File>>>,
    details: AcquisitionDetails,
}

impl DCMLocation {
    // pub fn read_channel(&self, channel: usize, region: &Region) -> Result<Vec<f32>, MCDError> {
    //     self.read_channels(&[channel], region)
    //         .map(|mut data| data.drain(..).last().unwrap())
    // }

    pub fn read_channels(
        &self,
        channels: &[usize],
        region: &Region,
    ) -> Result<Vec<Vec<f32>>, MCDError> {
        let mut data =
            vec![vec![0.0; region.width as usize * region.height as usize]; channels.len()];

        let mut reader = self.reader.lock().unwrap();

        let start_chunk_x = region.x / self.details.chunk_size;
        let end_chunk_x = ((region.x + region.width) / self.details.chunk_size + 1)
            .min(self.details.num_chunks_x());

        let start_chunk_y = region.y / self.details.chunk_size;
        let end_chunk_y = ((region.y + region.height) / self.details.chunk_size + 1)
            .min(self.details.num_chunks_y());

        let region_end_x = region.x + region.width;
        let region_end_y = region.y + region.height;

        for chunk_y in start_chunk_y..end_chunk_y {
            for chunk_x in start_chunk_x..end_chunk_x {
                let start_x = chunk_x * self.details.chunk_size;
                let end_x = (start_x + self.details.chunk_size).min(self.details.acquired_width());

                let start_y = chunk_y * self.details.chunk_size;
                let end_y = (start_y + self.details.chunk_size).min(self.details.acquired_height());

                let chunk_index = (chunk_y * self.details.num_chunks_x()) + chunk_x;

                let pixel_chunk = &self.details.chunks[chunk_index as usize];

                for (data, &channel) in data.iter_mut().zip(channels.iter()) {
                    let channel_chunk = &pixel_chunk.channels[channel];

                    let mut buf = vec![0; channel_chunk.length as usize];

                    reader.seek(SeekFrom::Start(channel_chunk.offset))?;
                    reader.read_exact(&mut buf)?;

                    let decompressed_data =
                        lz4_flex::decompress(&buf, channel_chunk.num_intensities as usize * 4)?;

                    let mut decompressed_data = Cursor::new(decompressed_data);

                    for y in start_y..end_y {
                        if region.y > y {
                            decompressed_data
                                .seek(SeekFrom::Current(self.details.chunk_size as i64 * 4))?;

                            continue;
                        }

                        if y >= region_end_y {
                            break;
                        }

                        let start_x = if region.x > start_x {
                            decompressed_data
                                .seek(SeekFrom::Current((region.x as i64 - start_x as i64) * 4))?;

                            region.x
                        } else {
                            start_x
                        };

                        for x in start_x..end_x {
                            // If we have gone past the end of the tile, then we can move on to the next line
                            if x >= region_end_x {
                                decompressed_data
                                    .seek(SeekFrom::Current((end_x as i64 - x as i64) * 4))?;

                                break;
                            }

                            // sometimes the run is stopped early, so make sure to check that we are loading in the desired data
                            if (y * self.details.acquired_width()) + x >= self.details.num_spectra {
                                break;
                            }

                            let intensity = decompressed_data.read_f32::<LittleEndian>()?;
                            let index = ((y - region.y) * region.width) + (x - region.x);

                            data[index as usize] = intensity;
                        }
                    }
                }
            }
        }

        Ok(data)
    }
}
