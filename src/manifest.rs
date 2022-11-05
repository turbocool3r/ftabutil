use crate::parser::Parser;
use serde::{
    de::{self, Unexpected, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::{fmt::Formatter, path::PathBuf};

pub struct TagVisitor;

impl<'de> Visitor<'de> for TagVisitor {
    type Value = Tag;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("either a non-negative integer less than 2^32 or a 4-byte string")
    }

    fn visit_i8<E>(self, v: i8) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_i64(v as i64)
    }

    fn visit_i16<E>(self, v: i16) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_i64(v as i64)
    }

    fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_i64(v as i64)
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if v < 0 {
            Err(E::invalid_value(
                Unexpected::Signed(v),
                &"a non-negative integer",
            ))
        } else if v > u32::MAX as i64 {
            Err(E::invalid_value(
                Unexpected::Signed(v),
                &"an integer less than 2^32",
            ))
        } else {
            Ok(Tag((v as u32).to_be_bytes()))
        }
    }

    fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_u64(v as u64)
    }

    fn visit_u16<E>(self, v: u16) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_u64(v as u64)
    }

    fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_u64(v as u64)
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if v > u32::MAX as u64 {
            Err(E::invalid_value(
                Unexpected::Unsigned(v),
                &"an integer less than 2^32",
            ))
        } else {
            Ok(Tag((v as u32).to_be_bytes()))
        }
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if v.len() <= 4 {
            let mut tag = [0u8; 4];
            tag[..v.len()].copy_from_slice(v.as_bytes());
            Ok(Tag(tag))
        } else {
            Err(E::invalid_length(v.len(), &"a string of 4 bytes or less"))
        }
    }

    fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_str(v)
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_str(&v)
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if v.len() <= 4 {
            let mut tag = [0u8; 4];
            tag[..v.len()].copy_from_slice(v);
            Ok(Tag(tag))
        } else {
            Err(E::invalid_length(
                v.len(),
                &"a byte string of 4 bytes or less",
            ))
        }
    }

    fn visit_borrowed_bytes<E>(self, v: &'de [u8]) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_bytes(v)
    }

    fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_bytes(&v)
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct Tag(pub [u8; 4]);

impl<'de> Deserialize<'de> for Tag {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(TagVisitor)
    }
}

impl Serialize for Tag {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if self.0.iter().all(|x| x.is_ascii_alphanumeric()) {
            let s = std::str::from_utf8(&self.0[..]).unwrap();
            serializer.serialize_str(s)
        } else {
            serializer.serialize_u32(u32::from_be_bytes(self.0))
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SegmentDesc {
    pub path: PathBuf,
    pub tag: Tag,
    pub unk: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub unk_0: u32,
    pub unk_1: u32,
    pub unk_2: u32,
    pub unk_3: u32,
    pub unk_4: u32,
    pub unk_5: u32,
    pub unk_6: u32,
    pub ticket: Option<PathBuf>,
    pub segments: Vec<SegmentDesc>,
}

impl Manifest {
    pub fn with_parser(parser: &Parser) -> Manifest {
        Manifest {
            unk_0: parser.unk_0(),
            unk_1: parser.unk_1(),
            unk_2: parser.unk_2(),
            unk_3: parser.unk_3(),
            unk_4: parser.unk_4(),
            unk_5: parser.unk_5(),
            unk_6: parser.unk_6(),
            ticket: None,
            segments: Vec::new(),
        }
    }
}
