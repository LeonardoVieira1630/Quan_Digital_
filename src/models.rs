// models.rs - Data Models

// This file contains data models and structures used across multiple files in your trading bot project.

// Structures:
// - KlineData: Represents the data of a candlestick (kline).
//   Properties:
//     - open_time: The opening timestamp of the candlestick.
//     - open: The opening price of the candlestick.
//     - high: The highest price reached during the candlestick period.
//     - low: The lowest price reached during the candlestick period.
//     - close: The closing price of the candlestick.
//     - volume: The trading volume during the candlestick period.
//     - close_time: The closing timestamp of the candlestick.
//     - quote_asset_volume: The volume of the quote asset traded during the candlestick period.
//     - number_of_trades: The total number of trades that occurred during the candlestick period.
//     - take_buy_base_asset_volume: The volume of the base asset bought during the candlestick period.
//     - take_buy_quote_asset_volume: The volume of the quote asset bought during the candlestick period.
//     - ignore: A property to ignore or discard (e.g., additional information not relevant to the candlestick data).

// The models.rs file serves as a centralized location to define the data structures used throughout your trading bot
// project. By encapsulating these structures in a separate file, it promotes code reusability, modularity, and
// consistent data representation.

// The KlineData structure represents the essential properties of a candlestick, including the open time, open price,
// high price, low price, close price, trading volume, close time, quote asset volume, number of trades, base asset
// volume bought, quote asset volume bought, and an ignored property. It provides a convenient way to store and access
// candlestick data in a structured manner.

use serde::{de, Deserialize, Deserializer, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct KlineData {
    pub open_time: i64,
    #[serde(deserialize_with = "de_float_from_str")]
    pub open: f64,
    #[serde(deserialize_with = "de_float_from_str")]
    pub high: f64,
    #[serde(deserialize_with = "de_float_from_str")]
    pub low: f64,
    #[serde(deserialize_with = "de_float_from_str")]
    pub close: f64,
    #[serde(deserialize_with = "de_float_from_str")]
    pub volume: f64,
    pub close_time: i64,
    #[serde(deserialize_with = "de_float_from_str")]
    pub quote_asset_volume: f64,
    pub number_of_trades: usize,
    #[serde(deserialize_with = "de_float_from_str")]
    pub take_buy_base_asset_volume: f64,
    #[serde(deserialize_with = "de_float_from_str")]
    pub take_buy_quote_asset_volume: f64,
    #[serde(deserialize_with = "de_float_from_str")]
    pub ignore: f64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct KlineDataString {
    pub open_time: i64,
    pub open: String,
    pub high: String,
    pub low: String,
    pub close: String,
    pub volume: String,
    pub close_time: i64,
    pub quote_asset_volume: String,
    pub number_of_trades: usize,
    pub take_buy_base_asset_volume: String,
    pub take_buy_quote_asset_volume: String,
    pub ignore: String,
}

pub fn de_float_from_str<'a, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: Deserializer<'a>,
{
    let str_val = String::deserialize(deserializer)?;
    str_val.parse::<f64>().map_err(de::Error::custom)
}
