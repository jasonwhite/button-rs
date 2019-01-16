// Copyright (c) 2018 Jason White
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

use std::fmt;
use std::fs;
use std::io;
use std::path::Path;

use failure::Fail;
use generic_array::{typenum, GenericArray};
use hex::{FromHex, FromHexError, ToHex};
use serde::{
    de::{self, Deserializer, Visitor},
    ser::{self, Serializer},
    Deserialize, Serialize,
};
use sha2::{self, Digest};

#[derive(Fail, Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct ShaVerifyError {
    expected: Sha256,
    found: Sha256,
}

impl ShaVerifyError {
    pub fn new(expected: Sha256, found: Sha256) -> ShaVerifyError {
        ShaVerifyError { expected, found }
    }
}

impl fmt::Display for ShaVerifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "expected SHA256 {}, but found {}",
            self.expected, self.found
        )
    }
}

/// Wrapper around a SHA256 value.
///
/// This can be serialized and deserialized as hex.
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct Sha256 {
    inner: GenericArray<u8, typenum::U32>,
}

impl Default for Sha256 {
    fn default() -> Sha256 {
        Sha256 {
            inner: sha2::Sha256::default().result(),
        }
    }
}

impl From<GenericArray<u8, typenum::U32>> for Sha256 {
    fn from(arr: GenericArray<u8, typenum::U32>) -> Self {
        Sha256 { inner: arr }
    }
}

impl Sha256 {
    pub fn from_reader<R>(mut reader: R) -> io::Result<Sha256>
    where
        R: io::Read,
    {
        let mut hasher = sha2::Sha256::default();

        const BUF_SIZE: usize = 16384;

        let mut buf = [0u8; BUF_SIZE];

        loop {
            let n = reader.read(&mut buf)?;

            if n == 0 {
                break;
            }

            hasher.input(&buf[0..n]);
        }

        Ok(Sha256 {
            inner: hasher.result(),
        })
    }

    pub fn from_path<P>(path: P) -> io::Result<Sha256>
    where
        P: AsRef<Path>,
    {
        Self::from_reader(fs::File::open(path.as_ref())?)
    }
}

impl fmt::Display for Sha256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:x}", self.inner)
    }
}

impl fmt::Debug for Sha256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:x}", self.inner)
    }
}

impl Serialize for Sha256 {
    /// Serialize a SHA256.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            // Serialize as a hex string.
            let mut hex = String::new();
            self.inner
                .as_ref()
                .write_hex(&mut hex)
                .map_err(ser::Error::custom)?;
            serializer.serialize_str(&hex)
        } else {
            // Serialize as a byte array with known length.
            serializer.serialize_bytes(self.inner.as_ref())
        }
    }
}

impl<'de> Deserialize<'de> for Sha256 {
    /// Deserialize a SHA256.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use std::marker::PhantomData;

        struct HexVisitor(PhantomData<Vec<u8>>);

        impl<'de> Visitor<'de> for HexVisitor {
            type Value = Sha256;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "hex string or bytes")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let v = <[u8; 32]>::from_hex(v).map_err(|e| match e {
                    FromHexError::InvalidHexCharacter { c, .. } => {
                        E::invalid_value(
                            de::Unexpected::Char(c),
                            &"string with only hexadecimal characters",
                        )
                    }
                    FromHexError::InvalidStringLength => E::invalid_length(
                        v.len(),
                        &"hex string with a valid length",
                    ),
                    FromHexError::OddLength => E::invalid_length(
                        v.len(),
                        &"hex string with an even length",
                    ),
                })?;

                Ok(Sha256 {
                    inner: GenericArray::from(v),
                })
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Sha256 {
                    inner: GenericArray::clone_from_slice(v),
                })
            }
        }

        if deserializer.is_human_readable() {
            // Deserialize from a hex string.
            deserializer.deserialize_str(HexVisitor(PhantomData))
        } else {
            // Deserialize from a byte array with known length.
            deserializer.deserialize_bytes(HexVisitor(PhantomData))
        }
    }
}
