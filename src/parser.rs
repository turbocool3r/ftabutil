pub mod error {
    use std::fmt;
    use thiserror::Error;

    #[derive(Error, Debug)]
    pub enum ParseError {
        #[error("file is too short to be a ftab file")]
        TooShort,
        #[error("file is not a ftab file (invalid magic value)")]
        UnknownMagic,
        #[error("segments list byte length is too large")]
        OverflowingSegmentsLength,
        #[error("segments list is larger than the space available in the file")]
        OobSegmentsList,
        #[error("ticket range in file is out of bounds")]
        OobTicket,
    }

    #[derive(Debug)]
    #[non_exhaustive]
    pub struct OobSegmentError {
        pub tag: [u8; 4],
    }

    impl fmt::Display for OobSegmentError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(
                f,
                "segment with tag {} is out of bounds",
                self.tag.escape_ascii()
            )
        }
    }
}

pub use error::{OobSegmentError, ParseError};
use std::slice;

/// Reads a 32-bit little-endian integer from the start of a byte slice and returns a tuple of the
/// slice's tail and the integer.
///
/// # Panics
/// Will panic if the slice is shorter than 4 bytes.
///
/// # Why not nom?
/// The previous implementation used nom for parsing, but it turned out to produce inefficient code.
#[inline(always)]
fn get_u32_le(bytes: &[u8]) -> (&[u8], u32) {
    let (bytes, tail) = bytes.split_at(4);
    let bytes: &[u8; 4] = bytes.try_into().unwrap();
    (tail, u32::from_le_bytes(*bytes))
}

#[inline(always)]
fn match_magic(bytes: &[u8]) -> Result<&[u8], ParseError> {
    let (head, tail) = bytes.split_at(8);
    if head == b"rkosftab" {
        Ok(tail)
    } else {
        Err(ParseError::UnknownMagic)
    }
}

#[derive(Clone, Debug)]
pub struct FtabParser<'a> {
    ticket: Option<&'a [u8]>,
    segments: &'a [[u8; 16]],
    tail: &'a [u8],
    unk_0: u32,
    unk_1: u32,
    unk_2: u32,
    unk_3: u32,
    unk_4: u32,
    unk_5: u32,
    unk_6: u32,
}

impl<'a> FtabParser<'a> {
    const HEADER_LEN: usize = 48;
    const SEGMENT_HEADER_LEN: usize = 16;

    pub fn from_bytes(bytes: &'a [u8]) -> Result<Self, ParseError> {
        if bytes.len() < Self::HEADER_LEN {
            return Err(ParseError::TooShort);
        }

        // Parse the header's fields.
        let (bytes, unk_0) = get_u32_le(bytes);
        let (bytes, unk_1) = get_u32_le(bytes);
        let (bytes, unk_2) = get_u32_le(bytes);
        let (bytes, unk_3) = get_u32_le(bytes);
        let (bytes, ticket_off) = get_u32_le(bytes);
        let (bytes, ticket_len) = get_u32_le(bytes);
        let (bytes, unk_4) = get_u32_le(bytes);
        let (bytes, unk_5) = get_u32_le(bytes);
        let bytes = match_magic(bytes)?;
        let (bytes, segments_cnt) = get_u32_le(bytes);
        let (tail, unk_6) = get_u32_le(bytes);

        // Calculate the lengths of the segments list and validate that it doesn't overflow and is
        // in bounds.
        let segments_cnt: usize = segments_cnt.try_into().unwrap();
        let segments_len = segments_cnt
            .checked_mul(Self::SEGMENT_HEADER_LEN)
            .ok_or(ParseError::OverflowingSegmentsLength)?;
        if segments_len > tail.len() {
            return Err(ParseError::OobSegmentsList);
        }

        debug!("Segments count is {}.", segments_cnt);

        // SAFETY: the length is verified not to overflow and to be less than the tail length. This
        // automatically implies that it's less than isize::MAX since this is also required for
        // tail.
        let segments_ptr =
            tail[..segments_len].as_ptr() as *const [u8; FtabParser::SEGMENT_HEADER_LEN];
        let segments = unsafe { slice::from_raw_parts(segments_ptr, segments_cnt) };

        // Ticket may or may not be present.
        let ticket = if ticket_off != 0 || ticket_len != 0 {
            debug!(
                "Ticket offset is {:#x}, length is {:#x}.",
                ticket_off, ticket_len
            );

            let ticket_off: usize = ticket_off.try_into().unwrap();
            let ticket_len: usize = ticket_len.try_into().unwrap();

            // Ensure that ticket's range is in bounds and also doesn't overflow.
            if ticket_off < Self::HEADER_LEN
                || ticket_off > bytes.len()
                || (bytes.len() - ticket_off) < ticket_len
            {
                return Err(ParseError::OobTicket);
            }
            let ticket_off = ticket_off - Self::HEADER_LEN;

            Some(&tail[ticket_off..ticket_off + ticket_len])
        } else {
            debug!("Ticket is not present.");

            None
        };

        Ok(Self {
            ticket,
            segments,
            tail: &tail[segments_len..],
            unk_0,
            unk_1,
            unk_2,
            unk_3,
            unk_4,
            unk_5,
            unk_6,
        })
    }

    #[inline]
    pub fn unk_0(&self) -> u32 {
        self.unk_0
    }

    #[inline]
    pub fn unk_1(&self) -> u32 {
        self.unk_1
    }

    #[inline]
    pub fn unk_2(&self) -> u32 {
        self.unk_2
    }

    #[inline]
    pub fn unk_3(&self) -> u32 {
        self.unk_3
    }

    #[inline]
    pub fn unk_4(&self) -> u32 {
        self.unk_4
    }

    #[inline]
    pub fn unk_5(&self) -> u32 {
        self.unk_5
    }

    #[inline]
    pub fn unk_6(&self) -> u32 {
        self.unk_6
    }

    pub fn ticket(&self) -> Option<&'a [u8]> {
        self.ticket
    }

    #[inline]
    pub fn segments(&self) -> SegmentsParser<'a> {
        SegmentsParser {
            headers: self.segments,
            data: self.tail,
            // This should be the initial length of the slice provided to the constructor so this
            // will never overflow.
            data_offset: self.segments.len() * Self::SEGMENT_HEADER_LEN + Self::HEADER_LEN,
        }
    }
}

#[derive(Clone, Debug)]
#[repr(C)]
pub struct SegmentHeader {
    pub tag: [u8; 4],
    pub seg_off: u32,
    pub seg_len: u32,
    pub unk: u32,
}

#[derive(Clone, Debug)]
pub struct ParsedSegment<'a> {
    pub tag: [u8; 4],
    pub data: &'a [u8],
    pub unk: u32,
}

#[derive(Clone, Debug)]
pub struct SegmentsParser<'a> {
    headers: &'a [[u8; 16]],
    data: &'a [u8],
    data_offset: usize,
}

impl<'a> SegmentsParser<'a> {
    pub fn next_segment(&mut self) -> Result<Option<ParsedSegment>, OobSegmentError> {
        let Some((bytes, tail)) = self.headers.split_first() else {
            return Ok(None);
        };

        let (tag, bytes) = bytes.split_at(4);
        let (bytes, offset) = get_u32_le(bytes);
        let (bytes, len) = get_u32_le(bytes);
        let (_, unk) = get_u32_le(bytes);

        // Extract the tag as a byte value.
        let tag: &[u8; 4] = tag.try_into().unwrap();
        let tag = *tag;

        // Validate offset and length and extract segment data.
        let offset: usize = offset.try_into().unwrap();
        let len: usize = len.try_into().unwrap();
        let data = self.data;
        let data_offset = self.data_offset;
        if offset < data_offset
            || (offset - data_offset) > data.len()
            || (data.len() - (offset - data_offset)) < len
        {
            return Err(OobSegmentError { tag });
        }
        let offset = offset - data_offset;
        let data = &data[offset..offset + len];

        self.headers = tail;

        Ok(Some(ParsedSegment { tag, data, unk }))
    }
}
