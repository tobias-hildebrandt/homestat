#![no_std]

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithTimestamp<Inner> {
    pub micros: u64,
    pub inner: Inner,
}
