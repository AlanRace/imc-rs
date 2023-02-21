struct Cell {
    markers: Vec<Summary<f32>>,
}

#[derive(Debug, Clone)]
struct Summary<T> {
    mean: T,
    std: T,
    range: (T, T),
    median: T,
}

struct Phenotype {
    description: String,
    rule: Rule,
}

impl Phenotype {
    pub fn matches(
        &self,
        channels: &[AcquisitionChannel],
        spectrum: &[Summary<f32>],
    ) -> Result<bool> {
        self.rule.matches(channels, spectrum)
    }
}

impl AsRef<Rule> for Phenotype {
    fn as_ref(&self) -> &Rule {
        &self.rule
    }
}

#[derive(Debug, Clone)]
enum Direction {
    Above,
    Below,
}

#[derive(Debug, Clone)]
enum Interval {
    Closed,
    Open,
}

#[derive(Debug, Clone)]
enum Rule {
    Threshold(ChannelIdentifier, f32, Direction, Interval),
    And(Box<Rule>, Box<Rule>),
    Or(Box<Rule>, Box<Rule>),
}

impl Rule {
    pub fn and<A: AsRef<Rule>, B: AsRef<Rule>>(left: A, right: B) -> Self {
        Self::And(
            Box::new(left.as_ref().clone()),
            Box::new(right.as_ref().clone()),
        )
    }

    pub fn matches(
        &self,
        channels: &[AcquisitionChannel],
        spectrum: &[Summary<f32>],
    ) -> Result<bool> {
        match self {
            Rule::Threshold(identifier, threshold, direction, interval) => {
                for (channel, summary) in channels.iter().zip(spectrum) {
                    if channel.is(identifier) {
                        match (direction, interval) {
                            (Direction::Above, Interval::Closed) => {
                                return Ok(summary.mean >= *threshold)
                            }
                            (Direction::Above, Interval::Open) => {
                                return Ok(summary.mean > *threshold)
                            }
                            (Direction::Below, Interval::Closed) => {
                                return Ok(summary.mean <= *threshold)
                            }
                            (Direction::Below, Interval::Open) => {
                                return Ok(summary.mean < *threshold)
                            }
                        }
                    }
                }

                // We didn't find the channel in the list of channels, so something went wrong
                Err(MCDError::InvalidChannel {
                    channel: identifier.clone(),
                })
            }
            Rule::And(left, right) => {
                Ok(left.matches(channels, spectrum)? && right.matches(channels, spectrum)?)
            }
            Rule::Or(left, right) => {
                Ok(left.matches(channels, spectrum)? || right.matches(channels, spectrum)?)
            }
        }
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_load() -> Result<()> {
        let filename = "../test/20200612_FLU_1923.mcd";

        println!("test_load() {:?}", filename);

        println!("{:?}", File::open(filename));
        let start = Instant::now();
        let mcd = MCD::from_path(filename)?;
        println!("Time taken to parse .mcd: {:?}", start.elapsed());

        // Optionally we can load/create the .dcm file for fast access to images
        let start = Instant::now();
        // let mcd = mcd.with_dcm()?;
        println!("Time taken to parse .dcm: {:?}", start.elapsed());

        let start = Instant::now();
        let roi_001 = mcd.acquisition("ROI_001").unwrap();
        println!("Time taken to find acquisition: {:?}", start.elapsed());

        let dna = roi_001.channel(ChannelIdentifier::label("DNA1")).unwrap();

        let start = Instant::now();
        let dna_roi001 = roi_001.channel_image(dna, None).unwrap();
        println!("Time taken to read channel image: {:?}", start.elapsed());

        let mut acq_image: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::new(dna_roi001.width(), dna_roi001.height());

        let mut index = 0;
        let max_value = 20.0;
        for y in 0..dna_roi001.height() {
            if index >= dna_roi001.valid_pixels {
                break;
            }

            for x in 0..dna_roi001.width() {
                if index >= dna_roi001.valid_pixels {
                    break;
                }

                let g = ((dna_roi001.data[index] / max_value) * 255.0) as u8;
                let g = g as f64 / 255.0;

                let cur_pixel = acq_image.get_pixel_mut(x as u32, y as u32).channels_mut();
                cur_pixel[1] = (g * 255.0) as u8;
                cur_pixel[3] = 255;

                index += 1;
            }
        }

        acq_image.save("dna.png").unwrap();

        // Available here: https://zenodo.org/record/4139443#.Y2okw0rMLmE

        let img_file = File::open("../test/20200612_FLU_1923-01_full_mask.tiff")?;
        let mut decoder = Decoder::new(img_file).expect("Cannot create decoder");

        let (width, height) = decoder.dimensions().unwrap();

        let mut cells: HashMap<u16, Vec<_>> = HashMap::new();
        let image = decoder.read_image().unwrap();

        match image {
            tiff::decoder::DecodingResult::U16(cell_data) => {
                for y in 0..height {
                    for x in 0..width {
                        let index = (y * width) + x;

                        if cell_data[index as usize] > 0 {
                            match cells.entry(cell_data[index as usize]) {
                                std::collections::hash_map::Entry::Occupied(mut entry) => {
                                    entry.get_mut().push((x, y));
                                }
                                std::collections::hash_map::Entry::Vacant(entry) => {
                                    entry.insert(vec![(x, y)]);
                                }
                            }
                        }
                    }
                }
            }
            _ => todo!(),
        }

        println!("Detected {} cells.", cells.len());
        println!("Time taken to detect cells: {:?}", start.elapsed());

        let cell = cells.get(&1).unwrap();

        println!(
            "{:?}",
            roi_001
                .channels()
                .iter()
                .map(|channel| channel.label())
                .collect::<Vec<_>>()
        );

        // cell types: https://github.com/camlab-bioml/astir/blob/master/tests/test-data/jackson-2020-markers.yml

        let phenotype_histone = Phenotype {
            description: "Histone+".to_string(),
            rule: Rule::Threshold(
                ChannelIdentifier::label("HistoneH3"),
                2.0,
                Direction::Above,
                Interval::Open,
            ),
        };

        let phenotype_cd16 = Phenotype {
            description: "CD16+".to_string(),
            rule: Rule::Threshold(
                ChannelIdentifier::label("CD16"),
                1.0,
                Direction::Above,
                Interval::Open,
            ),
        };

        let combined = Phenotype {
            description: "combined".to_string(),
            rule: Rule::and(&phenotype_histone, &phenotype_cd16),
        };

        for (index, cell) in cells {
            let mut spectrum = vec![Vec::with_capacity(cell.len()); roi_001.channels().len()];

            for (x, y) in cell {
                spectrum
                    .iter_mut()
                    .zip(roi_001.spectrum(x, y)?.iter())
                    .for_each(|(s, i)| s.push(*i));
            }

            let summaries = spectrum
                .drain(..)
                .map(|mut intensities| {
                    intensities.sort_by(|a, b| a.partial_cmp(b).unwrap());

                    // println!("{:?}", intensities);

                    let mean: f32 = intensities.iter().sum::<f32>() / intensities.len() as f32;

                    let variance: f32 =
                        intensities.iter().map(|x| (*x - mean).powi(2)).sum::<f32>()
                            / intensities.len() as f32;

                    let median = if intensities.len() % 2 == 0 {
                        let mid_point = intensities.len() / 2;

                        (intensities[mid_point] + intensities[mid_point - 1]) * 0.5
                    } else {
                        intensities[(intensities.len() - 1) / 2]
                    };

                    Summary {
                        mean,
                        median,
                        range: (intensities[0], intensities[intensities.len() - 1]),
                        std: variance.sqrt(),
                    }
                })
                .collect::<Vec<_>>();

            // println!("{:?}", summaries);
            // if combined.matches(roi_001.channels(), &spectrum) {
            // println!(
            //     "[{}] {:?} {:?} {:?}",
            //     index,
            //     phenotype_histone.matches(roi_001.channels(), &summaries),
            //     phenotype_cd16.matches(roi_001.channels(), &summaries),
            //     combined.matches(roi_001.channels(), &summaries)
            // );
            // }
        }

        // println!("{:?}", cell);

        Ok(())
    }
}
