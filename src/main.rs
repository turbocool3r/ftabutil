#[macro_use]
extern crate log;

mod builder;
mod format;
mod manifest;
mod parser;
mod util;

use crate::{
    builder::Builder,
    manifest::{Manifest, SegmentDesc, Tag},
};
use clap::{arg, command, value_parser, Command};
use log::LevelFilter;
use parser::FtabParser;
use simple_logger::SimpleLogger;
use std::{
    borrow::Cow,
    fs::{self, OpenOptions},
    io::{ErrorKind as IoErrorKind, Write},
    path::{Path, PathBuf},
};

fn save_file(name: &str, path: &Path, data: &[u8], overwrite: bool) -> Result<(), ()> {
    let result = OpenOptions::new()
        .write(true)
        .create_new(!overwrite)
        .create(overwrite)
        .truncate(overwrite)
        .open(path);

    match result {
        Ok(mut f) => match f.write_all(data) {
            Ok(()) => {
                info!("Saved {} to {}.", name, path.display());
                Ok(())
            }
            Err(e) => {
                error!("Couldn't save {} to {}: {}", name, path.display(), e);
                Err(())
            }
        },
        Err(e) => {
            error!("Couldn't create file at {}: {}", path.display(), e);
            Err(())
        }
    }
}

fn do_print_header(parser: &FtabParser) {
    println!("unk_0: {:#08x}", parser.unk_0());
    println!("unk_1: {:#08x}", parser.unk_1());
    println!("unk_2: {:#08x}", parser.unk_2());
    println!("unk_3: {:#08x}", parser.unk_3());
    println!("unk_4: {:#08x}", parser.unk_4());
    println!("unk_5: {:#08x}", parser.unk_5());
    println!("unk_6: {:#08x}", parser.unk_6());
}

fn main() {
    let matches = command!()
        .propagate_version(true)
        .subcommand_required(true)
        .arg_required_else_help(true)
        .arg(arg!(print_header: -H --print_header).help(
            "Prints fields of the ftab file header that are neither offsets nor magic \
                    (currently all are unknown and ignored).",
        ))
        .arg(
            arg!(log_level: -l --log_level <LEVEL>)
                .default_value("WARN")
                .help(
                    "Configures the log level for the tool. Available log levels are: NONE \
                    (disables logging entirely), TRACE, DEBUG, INFO, WARN and ERROR.",
                ),
        )
        .subcommand(
            Command::new("unpack")
                .arg(arg!(overwrite: -o --overwrite).help(
                    "Overwrite files instead of stopping when a file exists in the output \
                        directory.",
                ))
                .arg(
                    arg!(create_parent_dirs: -p --create_parent_dirs).help(
                        "Create parent directories when the output directory does not exist.",
                    ),
                )
                .arg(
                    arg!(in_file: <PATH>)
                        .value_parser(value_parser!(PathBuf))
                        .help("Path to the ftab file to be unpacked."),
                )
                .arg(
                    arg!(out_dir: [OUT_DIR])
                        .value_parser(value_parser!(PathBuf))
                        .help(
                            "Path to the directory where the unpacked files will be written. The \
                            default is the current working directory.",
                        ),
                )
                .about("Unpacks a ftab file into a directory."),
        )
        .subcommand(
            Command::new("pack")
                .arg(
                    arg!(manifest: <MANIFEST_PATH>)
                        .value_parser(value_parser!(PathBuf))
                        .help("Path to the manifest describing the desired ftab file."),
                )
                .arg(
                    arg!(out_file: [OUT_PATH])
                        .value_parser(value_parser!(PathBuf))
                        .help("Destination path where the created ftab file should be written."),
                )
                .about("Creates a ftab file from a manifest."),
        )
        .get_matches();

    let log_level: String = matches.get_one::<String>("log_level").unwrap().to_string();
    let log_level = match log_level.as_str() {
        "NONE" | "none" => LevelFilter::Off,
        "TRACE" | "trace" => LevelFilter::Trace,
        "DEBUG" | "debug" => LevelFilter::Debug,
        "INFO" | "info" => LevelFilter::Info,
        "WARN" | "warn" => LevelFilter::Warn,
        "ERROR" | "error" => LevelFilter::Error,
        _ => LevelFilter::Warn,
    };
    let print_header = matches.get_flag("print_header");

    SimpleLogger::new().with_level(log_level).init().unwrap();

    match matches.subcommand() {
        Some(("unpack", sub_matches)) => {
            let in_file: PathBuf = sub_matches.get_one::<PathBuf>("in_file").unwrap().clone();
            let out_dir: Option<PathBuf> =
                sub_matches.get_one::<PathBuf>("out_dir").map(Clone::clone);
            let overwrite = sub_matches.get_flag("overwrite");
            let create_parent_dirs = sub_matches.get_flag("create_parent_dirs");

            let data = match util::read_file(&in_file) {
                Ok(data) => data,
                Err(e) => {
                    error!("Couldn't open file at {}: {}", in_file.display(), e);
                    return;
                }
            };

            info!("Loaded file at path {}.", in_file.display());

            if let Some((p, e)) = out_dir.as_ref().and_then(|p| {
                if create_parent_dirs {
                    fs::create_dir_all(p)
                } else {
                    fs::create_dir(p)
                }
                .err()
                .map(|e| (p, e))
            }) {
                match e.kind() {
                    IoErrorKind::AlreadyExists => {
                        if !p.is_dir() {
                            error!("Path {} exists and is not a directory.", p.display());
                            return;
                        }
                    }
                    _ => {
                        error!("Couldn't create target directory at {}: {}", p.display(), e);
                        return;
                    }
                }
            }
            let out_dir = out_dir.unwrap_or_else(PathBuf::new);

            let parser = match FtabParser::from_bytes(&data) {
                Ok(parser) => parser,
                Err(e) => {
                    error!("Failed to parse file at {}: {}", in_file.display(), e);
                    return;
                }
            };

            let mut the_manifest = Manifest::with_parser(&parser);
            let mut manifest_path = out_dir.clone();
            manifest_path.push("manifest.toml");

            if print_header {
                do_print_header(&parser);
            }

            if let Some(ticket) = parser.ticket() {
                let mut filename = PathBuf::new();
                filename.push("ApImg4Ticket.der");

                let mut ticket_path = out_dir.clone();
                ticket_path.push(&filename);

                the_manifest.ticket = Some(filename);

                if save_file("ticket", &ticket_path, ticket, overwrite).is_err() {
                    return;
                }
            }

            let mut segments_parser = parser.segments();
            the_manifest.segments.reserve(segments_parser.count());
            loop {
                match segments_parser.next_segment() {
                    Ok(None) => {
                        let serialized_manifest = toml::to_vec(&the_manifest).unwrap();

                        if save_file("manifest", &manifest_path, &serialized_manifest, overwrite)
                            .is_err()
                        {
                            return;
                        }

                        info!("Done.");
                        break;
                    }
                    Ok(Some(segment)) => {
                        let filename = util::filename_for_tag(segment.tag);
                        let mut path = out_dir.clone();
                        path.push(&filename);

                        if save_file("segment", &path, segment.data, overwrite).is_err() {
                            return;
                        }

                        the_manifest.segments.push(SegmentDesc {
                            path: filename,
                            tag: Tag(segment.tag),
                            unk: segment.unk,
                        });
                    }
                    Err(e) => {
                        error!("Couldn't save segments: {}", e);
                    }
                }
            }
        }
        Some(("pack", sub_matches)) => {
            let manifest_path = sub_matches.get_one::<PathBuf>("manifest").unwrap();
            let manifest_data = match util::read_file(manifest_path) {
                Ok(data) => data,
                Err(e) => {
                    error!(
                        "Failed to read a manifest file at {}: {}",
                        manifest_path.display(),
                        e
                    );
                    return;
                }
            };

            let the_manifest = match toml::from_slice::<Manifest>(&manifest_data) {
                Ok(m) => m,
                Err(e) => {
                    error!(
                        "Failed to parse manifest file at {}: {}",
                        manifest_path.display(),
                        e
                    );
                    return;
                }
            };

            let input_dir = manifest_path.parent().unwrap();

            let out_file_path = sub_matches
                .get_one::<PathBuf>("out_file")
                .map(|p| Cow::from(p.as_path()))
                .unwrap_or_else(|| {
                    let mut path = manifest_path.parent().unwrap().to_path_buf();
                    path.push("ftab.bin");
                    Cow::from(path)
                });
            let mut out_file = match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&out_file_path)
            {
                Ok(f) => f,
                Err(e) => {
                    error!(
                        "Failed to create output file at {}: {}",
                        out_file_path.display(),
                        e
                    );
                    return;
                }
            };

            debug!("Writing ftab to {}.", out_file_path.display());

            match Builder::with_manifest(&the_manifest, input_dir)
                .and_then(|b| b.write_to(&mut out_file))
            {
                Ok(()) => info!("Done."),
                Err(e) => {
                    error!("An error occurred while building ftab file: {}", e);
                }
            }
        }
        Some(_) | None => unreachable!(),
    }
}
