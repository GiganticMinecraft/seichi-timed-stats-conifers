use anyhow::anyhow;
use bytes::buf::Buf;
use std::fmt::Debug;
use std::str::Utf8Error;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PlayerUuidString([u8; 36]);

impl PlayerUuidString {
    pub fn as_str(&self) -> Result<&str, Utf8Error> {
        std::str::from_utf8(&self.0)
    }

    pub fn from_string(str: &String) -> anyhow::Result<Self> {
        if !str.is_ascii() {
            Err(anyhow!("Expected ascii string for UuidString, got {str}"))
        } else if str.len() != 36 {
            Err(anyhow!(
                "Expect string of length 36 for UuidString, got {str}"
            ))
        } else {
            let mut result: [u8; 36] = [0; 36];
            str.as_bytes().copy_to_slice(result.as_mut_slice());
            Ok(Self(result))
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Player {
    pub uuid: PlayerUuidString,
}
