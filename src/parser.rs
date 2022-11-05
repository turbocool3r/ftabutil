//! Provides the [`Parser`] and [`SegmentParser`] structures that can be used to parse in-memory
//! 'ftab' files.

/// Provides [`ParseError`] and [`OobSegmentError`] that describe errors which may occur in
/// [`Parser::parse`] and [`SegmentParser::next_segment`] methods.
pub mod error {
    use std::{error::Error, fmt};
    use thiserror::Error;

    /// An error which may occur when parsing the 'ftab' file header.
    #[derive(Error, Debug)]
    pub enum ParseError {
        /// Returned when a file provided to a [`Parser`](../struct.Parser.html) is shorter than the
        /// size of a 'ftab' header.
        #[error("file is too short to be a ftab file")]
        TooShort,
        /// Returned when a file provided to a [`Parser`](../struct.Parser.html) does not contain
        /// the 'rkosftab' magic value at the specified offset.
        #[error("file is not a ftab file (invalid magic value)")]
        UnknownMagic,
        /// Returned when the product of the segments count from the 'ftab' header and the size of
        /// a segment list entry will overflow the size of the `usize` type.
        #[error("segments list byte length is too large")]
        OverflowingSegmentsLength,
        /// Returned when the end of the segments list exceeds past the end of a file.
        #[error("segments list is larger than the space available in the file")]
        OobSegmentsList,
        /// Returned when the range of a ticket exceeds past the end of a file or overlaps with
        /// either the header or the segment list.
        #[error("ticket range in file is out of bounds")]
        OobTicket,
    }

    /// Returned when the range of a 'ftab' file segment specified in its segment list entry exceeds
    /// past the end of the file or overlaps with either the header or the segment list.
    #[derive(Debug)]
    #[non_exhaustive]
    pub struct OobSegmentError {
        /// The tag specified in the segment list entry.
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

    impl Error for OobSegmentError {}
}

use crate::format::*;
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

/// Takes a subslice of a slice by a relative offset and length. The absolute offset in the slice is
/// determined by subtracting `slice_offset` from `offset`.
fn cut_subslice(slice: &[u8], offset: usize, len: usize, slice_offset: usize) -> Option<&[u8]> {
    let offset = offset.checked_sub(slice_offset)?;
    if offset <= slice.len() && (slice.len() - offset) >= len {
        Some(&slice[offset..offset + len])
    } else {
        None
    }
}

/// A parser that can be used to parse the 'ftab' file header and produce a [`SegmentsParser`].
#[derive(Clone, Debug)]
pub struct Parser<'a> {
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

impl<'a> Parser<'a> {
    /// Parse the provided byte slice as a 'ftab' file and return a [`Parser`].
    ///
    /// # Errors
    /// This method will return a [`ParseError`] in case the provided
    /// slice does not contain a valid 'ftab' file. For more info on the specific cases when this
    /// may happen see docs for individual [`ParseError`] variants.
    ///
    /// [`ParseError`]: error/enum.ParseError.html
    pub fn parse(bytes: &'a [u8]) -> Result<Self, ParseError> {
        if bytes.len() < HEADER_LEN {
            return Err(ParseError::TooShort);
        }

        // Parse the header's fields.
        let (bytes, unk_0) = get_u32_le(bytes);
        let (bytes, unk_1) = get_u32_le(bytes);
        let (bytes, unk_2) = get_u32_le(bytes);
        let (bytes, unk_3) = get_u32_le(bytes);
        let (bytes, ticket_offset) = get_u32_le(bytes);
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
            .checked_mul(SEGMENT_HEADER_LEN)
            .ok_or(ParseError::OverflowingSegmentsLength)?;
        if segments_len > tail.len() {
            return Err(ParseError::OobSegmentsList);
        }

        debug!("Segments count is {}.", segments_cnt);

        // SAFETY: the length is verified not to overflow and to be less than the tail length. This
        // automatically implies that it's less than isize::MAX since this is also required for
        // tail.
        let segments_ptr = tail[..segments_len].as_ptr() as *const [u8; SEGMENT_HEADER_LEN];
        let segments = unsafe { slice::from_raw_parts(segments_ptr, segments_cnt) };
        let tail = &tail[segments_len..];

        // Ticket may or may not be present.
        let ticket = if ticket_offset != 0 || ticket_len != 0 {
            debug!(
                "Ticket offset is {:#x}, length is {:#x}.",
                ticket_offset, ticket_len
            );

            let ticket_offset: usize = ticket_offset.try_into().unwrap();
            let ticket_len: usize = ticket_len.try_into().unwrap();

            // Ensure that ticket's range is in bounds and also doesn't overflow.
            let ticket = cut_subslice(tail, ticket_offset, ticket_len, HEADER_LEN + segments_len)
                .ok_or(ParseError::OobTicket)?;

            Some(ticket)
        } else {
            debug!("Ticket is not present.");

            None
        };

        Ok(Self {
            ticket,
            segments,
            tail,
            unk_0,
            unk_1,
            unk_2,
            unk_3,
            unk_4,
            unk_5,
            unk_6,
        })
    }

    /// Returns the `unk_0` field of the 'ftab' header. Its purpose is currently unknown.
    #[inline]
    pub fn unk_0(&self) -> u32 {
        self.unk_0
    }

    /// Returns the `unk_1` field of the 'ftab' header. Its purpose is currently unknown.
    #[inline]
    pub fn unk_1(&self) -> u32 {
        self.unk_1
    }

    /// Returns the `unk_2` field of the 'ftab' header. Its purpose is currently unknown.
    #[inline]
    pub fn unk_2(&self) -> u32 {
        self.unk_2
    }

    /// Returns the `unk_3` field of the 'ftab' header. Its purpose is currently unknown.
    #[inline]
    pub fn unk_3(&self) -> u32 {
        self.unk_3
    }

    /// Returns the `unk_4` field of the 'ftab' header. Its purpose is currently unknown.
    #[inline]
    pub fn unk_4(&self) -> u32 {
        self.unk_4
    }

    /// Returns the `unk_5` field of the 'ftab' header. Its purpose is currently unknown.
    #[inline]
    pub fn unk_5(&self) -> u32 {
        self.unk_5
    }

    /// Returns the `unk_6` field of the 'ftab' header. Its purpose is currently unknown.
    #[inline]
    pub fn unk_6(&self) -> u32 {
        self.unk_6
    }

    /// Returns the slice containing the APTicket included into the 'ftab' file or `None` if an
    /// APTicket is not included.
    pub fn ticket(&self) -> Option<&'a [u8]> {
        self.ticket
    }

    /// Returns a [`SegmentsParser`] over the segment list of the parsed 'ftab' file.
    #[inline]
    pub fn segments(&self) -> SegmentsParser<'a> {
        SegmentsParser {
            headers: self.segments,
            data: self.tail,
            // This should be the initial length of the slice provided to the constructor so this
            // will never overflow.
            data_offset: self.segments.len() * SEGMENT_HEADER_LEN + HEADER_LEN,
        }
    }
}

/// A description of a parsed segment.
#[derive(Clone, Debug)]
pub struct ParsedSegment<'a> {
    /// The segment's tag in the segment list.
    pub tag: [u8; 4],
    /// The segment's contents.
    pub data: &'a [u8],
    /// An field with a currently unknown purpose from the segment list entry.
    ///
    /// At the time of writing it seems to be ignored by software interpreting the format.
    pub unk: u32,
}

/// A parser for segment lists of 'ftab' files.
#[derive(Clone, Debug)]
pub struct SegmentsParser<'a> {
    headers: &'a [[u8; SEGMENT_HEADER_LEN]],
    data: &'a [u8],
    data_offset: usize,
}

impl<'a> SegmentsParser<'a> {
    /// Parses a segment list entry into a [`ParsedSegment`] and advances the parser to the next
    /// entry. Returns `None` when the last segment has been processed.
    ///
    /// # Errors
    /// This function will return an [`OobSegmentError`](error/struct.OobSegmentError.html) when
    /// a segment list entry is encountered which points outside the range of the file.
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

        let offset: usize = offset.try_into().unwrap();
        let len: usize = len.try_into().unwrap();

        // Validate offset and length and extract segment data.
        let data = cut_subslice(self.data, offset, len, self.data_offset)
            .ok_or(OobSegmentError { tag })?;

        self.headers = tail;

        Ok(Some(ParsedSegment { tag, data, unk }))
    }

    /// Returns the remaining count of the segment list to be parsed.
    pub fn count(&self) -> usize {
        self.headers.len()
    }
}
