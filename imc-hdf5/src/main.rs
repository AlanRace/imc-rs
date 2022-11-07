#[cfg(feature = "blosc")]
use hdf5::filters::blosc_set_nthreads;

use std::io::{BufRead, BufReader, Seek};

use hdf5::{File, Group, Location, Result};
use ndarray::{arr1, Array2};

use imc_rs::{ChannelIdentifier, OnSlide, OpticalImage, MCD};

pub fn create_str_attr(location: &Location, name: &str, value: &str) -> Result<()> {
    let attr = location
        .new_attr::<hdf5::types::VarLenUnicode>()
        .create(name)?;
    let value_: hdf5::types::VarLenUnicode = value.parse().unwrap();
    attr.write_scalar(&value_)
}

pub fn add_image<T: BufRead + Seek>(
    location: &Group,
    name: &str,
    image: OpticalImage<T>,
) -> Result<()> {
    let builder = location.new_dataset_builder();
    #[cfg(feature = "blosc")]
    let builder = builder.blosc_zstd(9, true); // zstd + shuffle

    let ds = builder
        .with_data(&arr1(&image.image_data().unwrap()))
        // finalize and write the dataset
        .create(name)?;

    create_str_attr(&ds, "type", &format!("{:?}", image.image_format()))?;

    Ok(())
}

fn write_hdf5<T: BufRead + Seek>(mcd: MCD<T>, name: &str) -> Result<()> {
    let file = File::create(format!("{}.h5", name))?; // open for writing

    #[cfg(feature = "blosc")]
    blosc_set_nthreads(2); // set number of blosc threads

    for slide in mcd.slides() {
        let slide_group = file.create_group(slide.description())?; // create a group

        add_image(&slide_group, "optical_image", slide.image())?;

        for panorama in slide.panoramas() {
            let panorama_group = slide_group.create_group(panorama.description())?;

            if let Some(panorama_image) = panorama.image() {
                add_image(&panorama_group, "optical_image", panorama_image)?;
            }

            for acquisition in panorama.acquisitions() {
                let acquisition_group = panorama_group.create_group(acquisition.description())?;

                let id_attr = acquisition_group.new_attr::<u16>().create("id")?;
                id_attr.write_scalar(&acquisition.id())?;

                let id_attr = acquisition_group
                    .new_attr::<f64>()
                    .create("ablation frequency")?;
                id_attr.write_scalar(&acquisition.ablation_frequency())?;

                let id_attr = acquisition_group.new_attr::<i16>().create("roi id")?;
                id_attr.write_scalar(&acquisition.acquisition_roi_id())?;

                let id_attr = acquisition_group.new_attr::<i32>().create("width")?;
                id_attr.write_scalar(&acquisition.width())?;
                let id_attr = acquisition_group.new_attr::<i32>().create("height")?;
                id_attr.write_scalar(&acquisition.height())?;

                let id_attr = acquisition_group
                    .new_attr::<usize>()
                    .create("num spectra")?;
                id_attr.write_scalar(&acquisition.num_spectra())?;

                let bounding_box = acquisition.slide_bounding_box();

                let id_attr = acquisition_group
                    .new_attr::<f64>()
                    .shape(2)
                    .create("slide top left (μm)")?;
                id_attr.write(&arr1(&[bounding_box.min_x, bounding_box.min_y]))?;

                let id_attr = acquisition_group
                    .new_attr::<f64>()
                    .shape(2)
                    .create("slide top right (μm)")?;
                id_attr.write(&arr1(&[
                    bounding_box.min_x + bounding_box.width,
                    bounding_box.min_y,
                ]))?;

                let id_attr = acquisition_group
                    .new_attr::<f64>()
                    .shape(2)
                    .create("slide bottom left (μm)")?;
                id_attr.write(&arr1(&[
                    bounding_box.min_x,
                    bounding_box.min_y + bounding_box.height,
                ]))?;

                let id_attr = acquisition_group
                    .new_attr::<f64>()
                    .shape(2)
                    .create("slide bottom right (μm)")?;
                id_attr.write(&arr1(&[
                    bounding_box.min_x + bounding_box.width,
                    bounding_box.min_y + bounding_box.height,
                ]))?;

                for channel in acquisition.channels() {
                    // We can skip the coordinates
                    if channel.label() == "X" || channel.label() == "Y" || channel.label() == "Z" {
                        continue;
                    }

                    let channel_image = acquisition
                        .channel_image(&ChannelIdentifier::Label(channel.label().to_string()), None)
                        .unwrap();

                    let builder = acquisition_group.new_dataset_builder();
                    #[cfg(feature = "blosc")]
                    let builder = builder.blosc_zstd(9, true); // zstd + shuffle

                    let image = Array2::from_shape_vec(
                        (
                            channel_image.height() as usize,
                            channel_image.width() as usize,
                        ),
                        channel_image.intensities().to_owned(),
                    )?;

                    let name = if channel.label().trim().is_empty() {
                        channel.name()
                    } else {
                        channel.label()
                    };

                    let ds = builder
                        .with_data(&image)
                        // finalize and write the dataset
                        .create(name)?;

                    create_str_attr(&ds, "label", channel.label())?;
                    create_str_attr(&ds, "name", channel.name())?;

                    let id_attr = ds.new_attr::<u16>().create("id")?;
                    id_attr.write_scalar(&channel.id())?;
                    let order_number_attr = ds.new_attr::<u16>().create("order number")?;
                    order_number_attr.write_scalar(&channel.order_number())?;
                }
            }
        }
    }

    // let ds = builder
    //     .with_data(&arr2(&[
    //         // write a 2-D array of data
    //         [Pixel::new(1, 2, R), Pixel::new(2, 3, B)],
    //         [Pixel::new(3, 4, G), Pixel::new(4, 5, R)],
    //         [Pixel::new(5, 6, B), Pixel::new(6, 7, G)],
    //     ]))
    //     // finalize and write the dataset
    //     .create("pixels")?;
    // // create an attr with fixed shape but don't write the data
    // let attr = ds.new_attr::<Color>().shape([3]).create("colors")?;
    // // write the attr data
    // attr.write(&[R, G, B])?;
    Ok(())
}

fn main() {
    let filename = "/media/alan/DATA/PuffPiece/AZ_NS_Puff piece slide_358_398_BCI.mcd";

    let file = BufReader::new(std::fs::File::open(filename).unwrap());
    let mcd = MCD::parse_with_dcm(file, filename).unwrap();

    write_hdf5(mcd, filename).unwrap();
}
