#![cfg_attr(not(feature = "full"), no_std)]

#[cfg(feature = "reflect")]
use reflect_instantiate::ReflectInstantiate;

#[cfg(feature = "full")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "full", derive(Serialize, Deserialize))]
#[cfg_attr(test, derive(PartialEq))]
#[cfg_attr(feature = "reflect", derive(ReflectInstantiate))]
pub struct FlashRegion {
    #[cfg_attr(feature = "full", serde(with = "full::serde_hexstring_bytes"))]
    /// Beginning address of region.
    pub origin: u32,
    #[cfg_attr(feature = "full", serde(with = "full::serde_hexstring_bytes"))]
    /// Size of flash region.
    pub length: u32,
}

impl FlashRegion {
    /// Create a byteslice of this region.
    ///
    /// # Safety
    /// The [`FlashRegion`] must be correct.
    pub unsafe fn as_slice(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self.origin as *const u8, self.length as usize) }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "full", derive(Serialize, Deserialize))]
#[cfg_attr(test, derive(PartialEq))]
#[cfg_attr(feature = "reflect", derive(ReflectInstantiate))]
pub struct Cyw43439Regions {
    // 43439a0 wifi firmware
    pub main: FlashRegion,
    // "Country Locale Matrix"
    // https://www.infineon.com/assets/row/public/documents/30/96/infineon-wi-fi-glossary-software-en.pdf
    pub clm: FlashRegion,
}

#[cfg(feature = "full")]
mod full {
    use super::*;

    use std::io::{Read, Write};

    /// One kibibyte
    const KIBIBYTE: u32 = 1024;

    const FLASH_ORIGIN: u32 = 0x1000_0000;
    const TOTAL_FLASH_SIZE: u32 = 2 * KIBIBYTE * KIBIBYTE;

    /// Must be synchronized with memory.x
    const CYW_MAIN_PREFIX: &str = "cyw43_main";
    const CYW_CLM_PREFIX: &str = "cyw43_clm";

    const ALIGNMENT: u32 = 4 * KIBIBYTE;

    fn align_down(address: u32) -> u32 {
        (address / ALIGNMENT) * ALIGNMENT
    }

    impl FlashRegion {
        fn write_linker_lines(
            &self,
            mut writer: impl Write,
            prefix: impl AsRef<str>,
        ) -> Result<(), std::io::Error> {
            writeln!(writer, "{}_origin = 0x{:x};", prefix.as_ref(), self.origin)?;
            writeln!(writer, "{}_length = 0x{:x};", prefix.as_ref(), self.length)?;
            Ok(())
        }
    }

    impl Cyw43439Regions {
        pub const JSON_FILENAME: &str = "flash-metadata.json";

        pub fn at_end(main_length: u32, clm_length: u32) -> Self {
            let clm_address = align_down(FLASH_ORIGIN + TOTAL_FLASH_SIZE - clm_length);
            let main_address = align_down(clm_address - main_length);
            Self {
                main: FlashRegion {
                    origin: main_address,
                    length: main_length,
                },
                clm: FlashRegion {
                    origin: clm_address,
                    length: clm_length,
                },
            }
        }

        pub fn write_linker(&self, mut writer: impl Write) -> Result<(), std::io::Error> {
            self.main.write_linker_lines(&mut writer, CYW_MAIN_PREFIX)?;
            self.clm.write_linker_lines(&mut writer, CYW_CLM_PREFIX)?;
            Ok(())
        }

        pub fn read_json(reader: impl Read) -> Result<Self, serde_json::Error> {
            serde_json::from_reader(reader)
        }

        pub fn write_json(&self, writer: impl Write) -> Result<(), serde_json::Error> {
            serde_json::to_writer_pretty(writer, self)
        }
    }

    pub mod serde_hexstring_bytes {
        use std::collections::VecDeque;

        use serde::{Deserialize, Deserializer, Serialize, Serializer};

        pub fn serialize<S>(val: &u32, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let hexstring = format!("0x{:x}", val);

            hexstring.serialize(serializer)
        }

        pub fn deserialize<'de, D>(deserializer: D) -> Result<u32, D::Error>
        where
            D: Deserializer<'de>,
        {
            let s = String::deserialize(deserializer)?;
            if s.len() < 3 || s.len() > 10 {
                return Err(serde::de::Error::custom(
                    "did not contain between 3 and 10 characters",
                ));
            }
            // Safety: checked length above
            let s = &s[2..];

            // make sure pairs for hex decode
            let s = if s.len() % 2 != 0 {
                format!("0{s}")
            } else {
                s.to_string()
            };
            let bytes = hex::decode(s).map_err(|e| serde::de::Error::custom(e.to_string()))?;
            let mut bytes = VecDeque::from(bytes);

            // push 0s to front to decode big endian
            while bytes.len() < 4 {
                bytes.push_front(0);
            }

            let slice = &bytes.make_contiguous()[0..4];
            // Safety: already checked length above
            let u = u32::from_be_bytes(slice.try_into().unwrap());
            Ok(u)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, stdout};

    use super::*;

    fn example() -> Cyw43439Regions {
        Cyw43439Regions::at_end(0x3ccea, 0x1290)
    }

    #[test]
    fn print_debug() {
        println!("debug:");
        println!("{:#x?}", example());
    }

    #[test]
    fn print_json() {
        println!("json:");
        example().write_json(&mut stdout()).unwrap();
        println!();
    }

    #[test]
    fn json_roundtrip() {
        let start = example();
        let mut out = Cursor::new(Vec::new());
        start.write_json(&mut out).unwrap();
        out.set_position(0);
        let new = Cyw43439Regions::read_json(&mut out).unwrap();

        assert_eq!(start, new);
    }

    #[test]
    fn print_linker() {
        println!("linker:");
        example().write_linker(&mut stdout()).unwrap();
        println!();
    }

    #[cfg(feature = "reflect")]
    #[test]
    fn print_reflect() {
        println!("reflect:");
        println!("{}", example().instantiate());
    }
}
