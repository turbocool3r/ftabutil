use std::path::PathBuf;
use std::{
    fs::File,
    io::{self, Read},
    path::Path,
};

pub fn read_file<P: AsRef<Path>>(path: P) -> Result<Vec<u8>, io::Error> {
    let mut f = File::open(path)?;
    let mut v = Vec::new();
    f.read_to_end(&mut v)?;
    Ok(v)
}

pub fn filename_for_tag(tag: [u8; 4]) -> PathBuf {
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
