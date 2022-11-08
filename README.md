# imc-rs

Library for accessing imaging mass cytometry (IMC) data stored in .mcd files. Access is provided to all channel data, metadata and optical images stored within the file. Additionally, it is possible to generate `slide overview images` which can be used in whole slide imaging registration workflows.

Written in Rust, with [Python bindings](bindings/python/README.md)


## Usage

IMC data in *.mcd files are stored in a spectrum-wise manner, in the order acquired on the instrument. This allows fast access to individual pixel information, but requires reading in all data from a single acquisition to generate a single channel image. 

This crate also provides an optional fast access to image data,as demonstrated below. This generates a temporary binary file in the same location as the .mcd file the first time this function is called (.dcm), which can take a few seconds. The temporary binary file is typically approximately 33% as large as the original .mcd file.

### With fast access to images

```rust
fn main() {
    let filename = "/location/to/data.mcd";
    let file = BufReader::new(File::open(filename).unwrap());
    let mcd = MCD::parse_with_dcm(file, filename);     
}
```