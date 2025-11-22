#![no_std]

use core::fmt::{Display, Formatter, Result as FmtResult};

use serde::{Deserialize, Serialize};

/// An number made up of a whole number plus some amount of tenths.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Number {
    pub whole: u8,  // TODO: 6 bits on wire (range of 64 degrees Celcius)
    pub tenths: u8, // TODO: 4 bits on wire (only need to encode 0-9)
}

/// A temperature and humidity reading.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reading {
    pub temperature: Number,
    pub humidity: Number,
}

impl Display for Reading {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(
            f,
            "{}.{}°C, {}.{}%RH",
            self.temperature.whole,
            self.temperature.tenths,
            self.humidity.whole,
            self.humidity.tenths,
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithTimestamp<Inner> {
    pub micros: u64,
    pub inner: Inner,
}

impl<Inner: Display> Display for WithTimestamp<Inner> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{:0>12}μs: {}", self.micros, self.inner)
    }
}

#[derive(Debug, thiserror::Error, Serialize, Deserialize)]
pub enum ReadError {
    #[error("Timeout at low start signal")]
    StartLowTimeout,
    #[error("Timeout at rising edge of start signal")]
    StartRisingTimeout,
    #[error("Timeout at falling edge of start signal")]
    StartFallingTimeout,
    #[error("Timeout at rising edge of data signal for bit {bit}")]
    DataRisingTimeout { bit: usize },
    #[error("Timeout at falling edge of data signal for bit {bit}")]
    DataFallingTimeout { bit: usize },
    #[error("{0}")]
    Checksum(#[from] ChecksumError),
}

#[derive(Debug, thiserror::Error, Serialize, Deserialize)]
#[error("Checksum error, expected {expected}, got {actual}")]
pub struct ChecksumError {
    pub expected: u8,
    pub actual: u8,
}

pub type WireMessage = WithTimestamp<Result<Reading, ReadError>>;

/// Helper type for [`Display`]ing a [`WireMessage`].
#[repr(transparent)]
#[derive(Debug)]
pub struct WireMessageDisplay<'a>(pub &'a WithTimestamp<Result<Reading, ReadError>>);

impl<'a> Display for WireMessageDisplay<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match &self.0.inner {
            Ok(reading) => write!(f, "{:0>12}μs: {}", self.0.micros, reading),
            Err(e) => write!(f, "{:0>12}μs: {}", self.0.micros, e),
        }
    }
}
