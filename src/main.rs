#[macro_use]
extern crate log;

mod parser;

use clap::{arg, command, value_parser, Command};
use log::LevelFilter;
use parser::FtabParser;
use simple_logger::SimpleLogger;
use std::{
    fs::{self, File, OpenOptions},
    io::{Error as IoError, ErrorKind as IoErrorKind, Read, Write},
    path::{Path, PathBuf},
};

fn read_file<P: AsRef<Path>>(path: P) -> Result<Vec<u8>, IoError> {
    let mut f = File::open(path)?;
    let mut v = Vec::new();
    f.read_to_end(&mut v)?;
    Ok(v)
}

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

fn path_for_tag(out_dir: &Path, tag: [u8; 4]) -> PathBuf {
    let mut path = out_dir.to_path_buf();

    let mut filename = if tag.iter().all(u8::is_ascii_alphanumeric) {
        String::from_utf8_lossy(&tag).into_owned()
    } else {
        hex::encode(tag)
    };
    filename.push_str(".bin");
    path.push(filename);

    path
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

            let data = match read_file(&in_file) {
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

            if print_header {
                do_print_header(&parser);
            }

            if let Some(ticket) = parser.ticket() {
                let mut ticket_path = out_dir.clone();
                ticket_path.push("ApImg4Ticket.der");

                if save_file("ticket", &ticket_path, ticket, overwrite).is_err() {
                    return;
                }
            }

            let mut segments_parser = parser.segments();
            loop {
                match segments_parser.next_segment() {
                    Ok(None) => {
                        info!("Done.");
                        break;
                    }
                    Ok(Some(segment)) => {
                        let path = path_for_tag(&out_dir, segment.tag);
                        if save_file("segment", &path, segment.data, overwrite).is_err() {
                            return;
                        }
                    }
                    Err(e) => {
                        error!("Couldn't save segments: {}", e);
                    }
                }
            }
        }
        Some(("info", _sub_matches)) => {}
        Some(_) | None => unreachable!(),
    }
}
