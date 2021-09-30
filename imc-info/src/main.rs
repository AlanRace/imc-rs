use clap::{AppSettings, Clap};
use imc_rs::MCD;

/// imc-info extracts information from IMC data sets stored in the *.mcd format.
#[derive(Clap)]
#[clap(version = "0.1", author = "Alan Race <alan.race@uni-marburg.de>")]
#[clap(setting = AppSettings::ColoredHelp)]
struct Opts {
    /// Sets a custom config file. Could have been an Option<T> with no default too
    #[clap(short, long, default_value = "default.conf")]
    config: String,
    /// *.mcd filename
    filename: String,
    /// A level of verbosity, and can be used multiple times
    #[clap(short, long, parse(from_occurrences))]
    verbose: i32,

    #[clap(subcommand)]
    slide_command: Option<SlideCommand>,
}

#[derive(Clap)]
enum SlideCommand {
    Slide(Slide),
}

/// A subcommand for controlling slides
#[derive(Clap)]
struct Slide {
    /// Print debug info
    id: u16,

    #[clap(subcommand)]
    panorama_command: Option<PanoramaCommand>,
}

#[derive(Clap)]
enum PanoramaCommand {
    Panorama(Panorama),
}

/// A subcommand for controlling panoramas
#[derive(Clap)]
struct Panorama {
    /// Print debug info
    id: u16,

    #[clap(subcommand)]
    acquisition_command: Option<AcquisitionCommand>,
}


#[derive(Clap)]
enum AcquisitionCommand {
    Acquisition(Acquisition),
}

/// A subcommand for controlling acquisition
#[derive(Clap)]
struct Acquisition {
    /// Print debug info
    id: u16,
}

fn main() {
    let opts: Opts = Opts::parse();

    // Gets a value for config if supplied by user, or defaults to "default.conf"
    //println!("Value for config: {}", opts.config);
    //println!("Using input file: {}", opts.filename);

    // Vary the output based on how many times the user used the "verbose" flag
    // (i.e. 'myprog -v -v -v' or 'myprog -vvv' vs 'myprog -v'
    /*match opts.verbose {
        0 => println!("No verbose info"),
        1 => println!("Some verbose info"),
        2 => println!("Tons of verbose info"),
        _ => println!("Don't be ridiculous"),
    }*/

    let file = std::fs::File::open(&opts.filename).unwrap();
    let mcd = MCD::parse(file, &opts.filename);

    // You can handle information about subcommands by requesting their matches by name
    // (as below), requesting just the name used, or both at the same time
    match opts.slide_command {
        Some(SlideCommand::Slide(slide_opts)) => {
            let slide = match mcd.slide(slide_opts.id) {
                Some(slide) => slide,
                None => {
                    println!("No such slide with ID {} (IDs are: {:?})", slide_opts.id, mcd.slide_ids());
                    return;
                }
            };

            match slide_opts.panorama_command {
                Some(PanoramaCommand::Panorama(panorama_opts)) => {
                    let panorama = match slide.panorama(&panorama_opts.id) {
                        Some(panorama) => panorama,
                        None => {
                            println!("No such panorama for slide {} with ID {} (IDs are: {:?})", slide.id(), panorama_opts.id, slide.panorama_ids());
                            return;
                        }
                    };

                    match panorama_opts.acquisition_command {
                        Some(AcquisitionCommand::Acquisition(acquisition_opts)) => {
                            let acquisition = match panorama.acquisition(&acquisition_opts.id) {
                                Some(acquisition) => acquisition,
                                None => {
                                    println!("No such acquisition for panorama {} with ID {} (IDs are: {:?})", panorama.id(), acquisition_opts.id, panorama.acquisition_ids());
                                    return;
                                }
                            };

                            println!("{}", acquisition);
                        },
                        None => {
                            println!("{}", panorama);
                        }
                    }
                }
                None => {
                    println!("{}", slide);
                }
            }
        },
        None => {
            println!("{}", mcd);
        }
    }

    // more program logic goes here...
}