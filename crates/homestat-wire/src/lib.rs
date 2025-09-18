#![no_std]

use serde::{Deserialize, Serialize};

/// An integer number and fraction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WholeAndDecimal {
    pub integer: u8,
    pub decimal: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Temperature(pub WholeAndDecimal);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Humidity(pub WholeAndDecimal);

/// A temperature and humidity reading.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reading {
    pub temperature: Temperature,
    pub humidity: Humidity,
}
