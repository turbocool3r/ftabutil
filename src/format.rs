use std::mem;

pub const HEADER_LEN: usize = mem::size_of::<FtabHeader>();
pub const SEGMENT_HEADER_LEN: usize = mem::size_of::<SegmentHeader>();

#[derive(Clone, Debug)]
#[repr(C)]
pub struct FtabHeader {
    pub unk_0: u32,
    pub unk_1: u32,
    pub unk_2: u32,
    pub unk_3: u32,
    pub ticket_offset: u32,
    pub ticket_len: u32,
    pub unk_4: u32,
    pub unk_5: u32,
    pub magic: [u8; 8],
    pub segments_count: u32,
    pub unk_6: u32,
}

#[derive(Clone, Debug)]
#[repr(C)]
pub struct SegmentHeader {
    pub tag: [u8; 4],
    pub seg_off: u32,
    pub seg_len: u32,
    pub unk: u32,
}
