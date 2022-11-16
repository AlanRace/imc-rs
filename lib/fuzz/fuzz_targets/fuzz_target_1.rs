#![no_main]

use std::io::Cursor;

use imc_rs::ChannelIdentifier;
use imc_rs::MCD;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // fuzzed code goes here

    let data = Cursor::new(data);

    let mcd = MCD::parse(data);

    if let Ok(mcd) = mcd {
        for slide in mcd.slides() {
            for panorama in slide.panoramas() {
                for acquisition in panorama.acquisitions() {
                    for channel in acquisition.channels() {
                        acquisition.channel_image(
                            &ChannelIdentifier::Name(channel.name().to_string()),
                            None,
                        );
                    }
                }
            }
        }
    }
});
