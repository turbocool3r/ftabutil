//! Provides the [`Builder`] structure that can be used to build 'ftab' files from
//! [`Manifest`](../manifest/struct.Manifest.html)s.

use crate::{error::FileOpError, format::*, manifest::Manifest, util};
use std::{
    io::{self, Write},
    mem,
    path::Path,
    slice,
};

/// A builder that can be used to build 'ftab' files from
/// [`Manifest`](../manifest/struct.Manifest.html)s.
///
/// Currently it does not have API to build 'ftab' files from scratch as it is not required.
#[derive(Default, Clone, Debug)]
pub struct Builder {
    segments: Vec<SegmentHeader>,
    data: Vec<u8>,
    ticket: Option<Box<[u8]>>,
    unk_0: u32,
    unk_1: u32,
    unk_2: u32,
    unk_3: u32,
    unk_4: u32,
    unk_5: u32,
    unk_6: u32,
}

impl Builder {
    /// Creates a [`Builder`] and fills it using a description from a
    /// [`Manifest`](../manifest/struct.Manifest.html).
    ///
    /// # Errors
    /// Returns a boxed [`FileOpError`](../error/struct.FileOpError.html) error when one of the
    /// files from the manifest's segments lists fails to load.
    pub fn with_manifest(
        manifest: &Manifest,
        dir: Option<&Path>,
    ) -> Result<Self, Box<FileOpError>> {
        let mut data_offset = HEADER_LEN + manifest.segments.len() * SEGMENT_HEADER_LEN;
        let mut segments = Vec::with_capacity(manifest.segments.len());
        let mut data = Vec::new();

        for segment in manifest.segments.iter() {
            debug!(
                "Reading segment with tag {} from file at {}.",
                segment.tag.0.escape_ascii(),
                segment.path.display()
            );

            let path = util::qualify_path_if_needed(&segment.path, dir);
            let segment_data = util::read_file("segment", path)?;

            // This will not pad the ticket, but that's how the original ftab builder seems to work
            // so we do it this way.
            let padding = (4 - data.len() % 4) % 4;
            data.resize(data.len() + padding, 0);
            data_offset += padding;

            trace!(
                "Segment offset is {}, length is {}.",
                data_offset,
                segment_data.len()
            );

            segments.push(SegmentHeader {
                tag: segment.tag.0,
                seg_off: data_offset.try_into().unwrap(),
                seg_len: segment_data.len().try_into().unwrap(),
                unk: 0,
            });
            data.extend_from_slice(&segment_data);

            trace!("Padded with {} null bytes.", padding);

            data_offset += segment_data.len();
        }

        let ticket = if let Some(rel_path) = manifest.ticket.as_ref() {
            let path = util::qualify_path_if_needed(rel_path, dir);
            Some(util::read_file("ticket", path).map(Vec::into_boxed_slice)?)
        } else {
            None
        };

        Ok(Self {
            segments,
            data,
            ticket,
            unk_0: manifest.unk_0,
            unk_1: manifest.unk_1,
            unk_2: manifest.unk_2,
            unk_3: manifest.unk_3,
            unk_4: manifest.unk_4,
            unk_5: manifest.unk_5,
            unk_6: manifest.unk_6,
        })
    }

    /// Writes the built 'ftab' into anything implementing the `std::io::Write` trait.
    ///
    /// # Errors
    /// Returns an I/O error in case it ever occurs.
    pub fn write_to<W: Write>(&self, dest: &mut W) -> io::Result<()> {
        let data_offset = HEADER_LEN + self.segments.len() * SEGMENT_HEADER_LEN;
        let header = FtabHeader {
            unk_0: self.unk_0,
            unk_1: self.unk_1,
            unk_2: self.unk_2,
            unk_3: self.unk_3,
            ticket_offset: self
                .ticket
                .as_ref()
                .map(|_| data_offset + self.data.len())
                .unwrap_or(0)
                .try_into()
                .unwrap(),
            ticket_len: self
                .ticket
                .as_ref()
                .map(|x| x.len())
                .unwrap_or(0)
                .try_into()
                .unwrap(),
            unk_4: self.unk_4,
            unk_5: self.unk_5,
            magic: *b"rkosftab",
            segments_count: self.segments.len().try_into().unwrap(),
            unk_6: self.unk_6,
        };

        // This is safe because of repr(C) and no padding.
        let header_bytes: &[u8; HEADER_LEN] = unsafe { mem::transmute(&header) };
        let segment_list_bytes: &[u8] = unsafe {
            slice::from_raw_parts(
                self.segments.as_ptr() as *const u8,
                self.segments.len() * SEGMENT_HEADER_LEN,
            )
        };

        dest.write_all(header_bytes)?;
        dest.write_all(segment_list_bytes)?;
        dest.write_all(&self.data)?;

        if let Some(ticket) = self.ticket.as_deref() {
            dest.write_all(ticket)?;
        }

        Ok(())
    }
}
