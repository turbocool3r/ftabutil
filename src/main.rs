#[macro_use]
extern crate log;

mod builder;
mod error;
mod format;
mod manifest;
mod parser;
mod util;

use crate::{
    builder::Builder,
    error::{FileOpError, PackError, UnpackError},
    manifest::{Manifest, SegmentDesc, Tag},
    parser::Parser,
};
use clap::{arg, command, value_parser, Command};
use log::LevelFilter;
use simple_logger::SimpleLogger;
use std::{
    fs,
    io::ErrorKind as IoErrorKind,
    path::{Path, PathBuf},
};

fn do_print_header(parser: &Parser) {
    println!("unk_0: {:#08x}", parser.unk_0());
    println!("unk_1: {:#08x}", parser.unk_1());
    println!("unk_2: {:#08x}", parser.unk_2());
    println!("unk_3: {:#08x}", parser.unk_3());
    println!("unk_4: {:#08x}", parser.unk_4());
    println!("unk_5: {:#08x}", parser.unk_5());
    println!("unk_6: {:#08x}", parser.unk_6());
}

fn filename_for_tag(tag: [u8; 4]) -> PathBuf {
    let filename = if tag.iter().all(u8::is_ascii_alphanumeric) {
        let tag_str = std::str::from_utf8(&tag).unwrap();
        format!("{}.bin", tag_str)
    } else {
        format!("tag_{}.bin", hex::encode(tag))
    };

    let mut path = PathBuf::new();
    path.push(filename);

    path
}

fn do_unpack<'a>(
    in_file: &'a Path,
    out_dir: Option<&'a Path>,
    overwrite: bool,
    create_parent_dirs: bool,
    print_header: bool,
) -> Result<(), UnpackError<'a>> {
    use UnpackError::*;

    let data = util::read_file("input file", in_file)?;

    info!("Loaded file at path {}.", in_file.display());

    if let Some(out_dir) = out_dir {
        if create_parent_dirs {
            fs::create_dir_all(out_dir)
        } else {
            fs::create_dir(out_dir)
        }
        .or_else(|e| match e.kind() {
            IoErrorKind::AlreadyExists => {
                if !out_dir.is_dir() {
                    Err(OutDirIsNotDir(out_dir))
                } else {
                    Ok(())
                }
            }
            _ => Err(FailedToCreateOutDir(out_dir, e)),
        })?;
    }

    // Parse the header and initialize the parser.
    let parser = Parser::parse(&data).map_err(|e| HeaderParseError(in_file, e))?;

    let mut the_manifest = Manifest::with_parser(&parser);
    let manifest_path = util::qualify_path_if_needed("manifest.toml", out_dir);

    if print_header {
        do_print_header(&parser);
    }

    if let Some(ticket) = parser.ticket() {
        let mut filename = PathBuf::new();
        filename.push("ApImg4Ticket.der");

        let ticket_path = util::qualify_path_if_needed(&filename, out_dir);
        util::save_file("ticket", ticket_path, ticket, overwrite)?;

        the_manifest.ticket = Some(filename);
    }

    let mut segments_parser = parser.segments();
    the_manifest.segments.reserve(segments_parser.count());
    loop {
        match segments_parser.next_segment()? {
            None => {
                let serialized_manifest = toml::to_vec(&the_manifest).unwrap();
                util::save_file("manifest", manifest_path, &serialized_manifest, overwrite)?;

                info!("Done.");

                break Ok(());
            }
            Some(segment) => {
                let filename = filename_for_tag(segment.tag);
                let path = util::qualify_path_if_needed(&filename, out_dir);

                util::save_file("segment", path, segment.data, overwrite)?;

                the_manifest.segments.push(SegmentDesc {
                    path: filename,
                    tag: Tag(segment.tag),
                    unk: segment.unk,
                });
            }
        }
    }
}

fn do_pack<'a>(
    manifest_path: &'a Path,
    out_path: Option<&'a Path>,
    overwrite: bool,
) -> Result<(), PackError<'a>> {
    use PackError::*;

    // read and parse the manifest ensuring that the parent directory in the manifest's path exists
    let manifest_data = util::read_file("manifest", manifest_path)?;
    let the_manifest = toml::from_slice::<Manifest>(&manifest_data)
        .map_err(|e| ManifestParseError(manifest_path, e))?;

    // create the output file
    let input_dir = manifest_path.parent();
    let out_file_path = util::qualify_path_or_default_if_needed(out_path, input_dir, "ftab.bin");
    let mut out_file = util::create_file("output file", &out_file_path, overwrite)?;

    debug!("Writing ftab to {}.", out_file_path.display());

    // make a builder from the manifest and build the ftab file
    let builder = Builder::with_manifest(&the_manifest, input_dir)?;
    builder.write_to(&mut out_file).map_err(|error| {
        FileOpError::make_write("output file", out_file_path.to_path_buf(), error)
    })?;

    info!("Done.");

    Ok(())
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
                .arg(arg!(overwrite: -o --overwrite).help(
                    "Overwrites the output file instead of stopping when the file exists at the \
                    specified path.",
                ))
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

            if let Err(e) = do_unpack(
                &in_file,
                out_dir.as_deref(),
                overwrite,
                create_parent_dirs,
                print_header,
            ) {
                error!("{}", e);
            }
        }
        Some(("pack", sub_matches)) => {
            let manifest_path = sub_matches.get_one::<PathBuf>("manifest").unwrap();
            let out_file = sub_matches
                .get_one::<PathBuf>("out_file")
                .map(PathBuf::as_path);
            let overwrite = sub_matches.get_flag("overwrite");

            if let Err(e) = do_pack(manifest_path, out_file, overwrite) {
                error!("{}", e);
            }
        }
        Some(_) | None => unreachable!(),
    }
}
