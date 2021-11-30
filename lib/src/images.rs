use std::{
    convert::TryInto,
    io::{Read, Seek, SeekFrom},
};

pub(crate) fn read_image_data<T: Read + Seek>(
    mut source: std::sync::MutexGuard<T>,
    start_offset: i64,
    end_offset: i64,
) -> std::io::Result<Vec<u8>> {
    let mut image_start_offset = start_offset;

    // Add an offset to skip the C# Drawing data
    image_start_offset += 161;
    let image_size = end_offset - image_start_offset;

    let mut buf_u8 = vec![0; image_size.try_into().unwrap()];

    match source.seek(SeekFrom::Start(image_start_offset as u64)) {
        Ok(_seek) => match source.read_exact(&mut buf_u8) {
            Ok(()) => Ok(buf_u8),
            Err(error) => Err(error),
        },
        Err(error) => Err(error),
    }
}
