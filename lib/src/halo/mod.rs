use std::{
    collections::HashMap,
    io::{BufReader, Read, Seek},
    path::Path,
};

use crate::BoundingBox;

/// Parse output from HALO cell detection stored as a .csv file, returning a representation of the cell data
pub fn parse_from_path<P: AsRef<Path>>(path: P) -> std::io::Result<CellData> {
    //parse_from_path(path.as_ref())
    let file = std::fs::File::open(path)?;
    let reader = BufReader::new(file);

    parse(reader)
}

#[derive(Clone, Copy, Debug)]
enum ColumnType {
    Text,
    Binary,
    Integer,
    Float,
}

/// Represents the data stored in a column. Assumes that all data is of the same type.
/// Strings are stored as a DictionaryID as most strings appear multiple times.
#[derive(Debug)]
pub enum ColumnData {
    /// Text data, stored as a DictionaryID
    Text(Vec<DictionaryID>),
    /// Binary column data
    Binary(Vec<bool>),
    /// Integer column data
    Integer(Vec<i64>),
    /// Floating point column data
    Float(Vec<f64>),
}

/// Describes a column in the .csv file
#[derive(Debug)]
pub struct Column {
    description: ColumnDescription,
    number: usize,
}

impl Column {
    /// Returns the name (title) of the column
    pub fn name(&self) -> &str {
        &self.description.name
    }

    /// Returns the index of the column (number representing the order in which the column appears)
    pub fn column_number(&self) -> usize {
        self.number
    }
}

#[derive(Debug)]
struct ColumnDescription {
    name: String,
    column_type: ColumnType,
}

struct KnownColumnDescription {
    name: &'static str,
    column_type: ColumnType,
}

const KNOWN_HEADERS: &[KnownColumnDescription] = &[
    KnownColumnDescription {
        name: "Image Location",
        column_type: ColumnType::Text,
    },
    KnownColumnDescription {
        name: "Analysis Region",
        column_type: ColumnType::Text,
    },
    KnownColumnDescription {
        name: "Analysis Inputs",
        column_type: ColumnType::Text,
    },
    KnownColumnDescription {
        name: "Object Id",
        column_type: ColumnType::Integer,
    },
    KnownColumnDescription {
        name: "XMin",
        column_type: ColumnType::Integer,
    },
    KnownColumnDescription {
        name: "XMax",
        column_type: ColumnType::Integer,
    },
    KnownColumnDescription {
        name: "YMin",
        column_type: ColumnType::Integer,
    },
    KnownColumnDescription {
        name: "YMax",
        column_type: ColumnType::Integer,
    },
    KnownColumnDescription {
        name: "Cell Area (µm²)",
        column_type: ColumnType::Integer,
    },
    KnownColumnDescription {
        name: "Cytoplasm Area (µm²)",
        column_type: ColumnType::Integer,
    },
    KnownColumnDescription {
        name: "Nucleus Area (µm²)",
        column_type: ColumnType::Integer,
    },
    KnownColumnDescription {
        name: "Nucleus Perimeter (µm)",
        column_type: ColumnType::Integer,
    },
    KnownColumnDescription {
        name: "Nucleus Roundness",
        column_type: ColumnType::Float,
    },
    KnownColumnDescription {
        name: "Classifier Label",
        column_type: ColumnType::Text,
    },
];

type DictionaryID = usize;

#[derive(Debug)]
struct Dictionary {
    by_value: HashMap<String, DictionaryID>,
    by_id: HashMap<DictionaryID, String>,

    next_id: DictionaryID,
}

impl Dictionary {
    pub fn new() -> Self {
        Dictionary {
            by_value: HashMap::new(),
            by_id: HashMap::new(),
            next_id: 0,
        }
    }

    pub fn get_or_insert(&mut self, entry: &str) -> DictionaryID {
        match self.by_value.get(entry) {
            Some(id) => *id,
            None => {
                let id = self.next_id;
                self.by_value.insert(entry.to_string(), id);
                self.by_id.insert(id, entry.to_string());

                self.next_id += 1;

                id
            }
        }
    }
}

/// Represents cell segmentation and analysis data parsed from .csv file
#[allow(dead_code)]
pub struct CellData {
    headers: Vec<Column>,
    data: Vec<ColumnData>,
    dictionary: Dictionary,
}

// TODO: Allow selecting column based on Header, Index, HeaderContains,...

impl CellData {
    /// Returns a header `Column` with the specified name
    pub fn header(&self, name: &str) -> Option<&Column> {
        for header in &self.headers {
            if header.description.name == name {
                return Some(header);
            }
        }

        None
    }

    /// Returns the data in the column at the ith position in the file
    pub fn column_data(&self, index: usize) -> Option<&ColumnData> {
        if index < self.data.len() {
            Some(&self.data[index])
        } else {
            None
        }
    }

    /// Returns an iterator over each cell, providing the detected boundaries for each cell
    pub fn boundaries(&self) -> BoundariesIterator {
        let x_min_header = self.header("XMin").unwrap();
        let x_min_data = self.column_data(x_min_header.column_number()).unwrap();
        let x_max_header = self.header("XMax").unwrap();
        let x_max_data = self.column_data(x_max_header.column_number()).unwrap();

        let y_min_header = self.header("YMin").unwrap();
        let y_min_data = self.column_data(y_min_header.column_number()).unwrap();
        let y_max_header = self.header("YMax").unwrap();
        let y_max_data = self.column_data(y_max_header.column_number()).unwrap();

        if let ColumnData::Integer(x_min_data) = x_min_data {
            if let ColumnData::Integer(x_max_data) = x_max_data {
                if let ColumnData::Integer(y_min_data) = y_min_data {
                    if let ColumnData::Integer(y_max_data) = y_max_data {
                        return BoundariesIterator {
                            x_min_data,
                            x_max_data,
                            y_min_data,
                            y_max_data,

                            index: 0,
                        };
                    }
                }
            }
        }

        panic!("Failed to create boundaries iterator")
    }
}

/// Iterator over each cell, providing the detected boundaries for each cell
pub struct BoundariesIterator<'a> {
    x_min_data: &'a Vec<i64>,
    x_max_data: &'a Vec<i64>,
    y_min_data: &'a Vec<i64>,
    y_max_data: &'a Vec<i64>,

    index: usize,
}

impl<'a> Iterator for BoundariesIterator<'a> {
    type Item = BoundingBox<i64>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.x_min_data.len() {
            return None;
        }

        let result = BoundingBox {
            min_x: self.x_min_data[self.index],
            min_y: self.y_min_data[self.index],
            width: self.x_max_data[self.index] - self.x_min_data[self.index],
            height: self.y_max_data[self.index] - self.y_min_data[self.index],
        };
        self.index += 1;
        Some(result)
    }
}

/// Parse output from HALO cell detection stored as a .csv file, returning a representation of the cell data
pub fn parse<R: Read + Seek>(reader: R) -> std::io::Result<CellData> {
    let mut rdr = csv::Reader::from_reader(reader);

    let mut dictionary = Dictionary::new();

    // Create a hashmap with known columns
    let mut known_columns_map = HashMap::with_capacity(KNOWN_HEADERS.len());
    for known_header in KNOWN_HEADERS {
        known_columns_map.insert(known_header.name.to_string(), known_header);
    }

    let header_records = rdr.headers()?;

    let mut headers = Vec::with_capacity(header_records.len());
    let mut column_data = Vec::with_capacity(header_records.len());

    for (index, header) in header_records.iter().enumerate() {
        if let Some(known_header) = known_columns_map.get(header) {
            headers.push(Column {
                description: ColumnDescription {
                    name: known_header.name.to_string(),
                    column_type: known_header.column_type,
                },
                number: index,
            });
        } else if header.contains("Positive") {
            headers.push(Column {
                description: ColumnDescription {
                    name: header.to_string(),
                    column_type: ColumnType::Binary,
                },
                number: index,
            });
        } else if header.contains("Intensity") {
            headers.push(Column {
                description: ColumnDescription {
                    name: header.to_string(),
                    column_type: ColumnType::Float,
                },
                number: index,
            });
        } else {
            headers.push(Column {
                description: ColumnDescription {
                    name: header.to_string(),
                    column_type: ColumnType::Binary,
                },
                number: index,
            });
        }

        match headers.last().unwrap().description.column_type {
            ColumnType::Binary => column_data.push(ColumnData::Binary(Vec::new())),
            ColumnType::Text => column_data.push(ColumnData::Text(Vec::new())),
            ColumnType::Integer => column_data.push(ColumnData::Integer(Vec::new())),
            ColumnType::Float => column_data.push(ColumnData::Float(Vec::new())),
        }
    }

    for result in rdr.records() {
        // The iterator yields Result<StringRecord, Error>, so we check the
        // error here.
        let record = result?;

        for (index, (entry, column_data)) in record.iter().zip(column_data.iter_mut()).enumerate() {
            match column_data {
                ColumnData::Text(data) => {
                    let id = dictionary.get_or_insert(entry);
                    data.push(id);
                }
                ColumnData::Binary(data) => data.push(entry.parse::<u8>().unwrap() == 1),
                ColumnData::Integer(data) => {
                    let parse_result = entry.parse::<i64>();

                    match parse_result {
                        Ok(value) => data.push(value),
                        Err(error) => {
                            panic!(
                                "Failed to parse integer {} for column {:?} [{}]",
                                entry, headers[index], error
                            );
                        }
                    }
                }
                ColumnData::Float(data) => data.push(entry.parse::<f64>().unwrap()),
            };
        }
    }

    Ok(CellData {
        headers,
        data: column_data,
        dictionary,
    })
}
