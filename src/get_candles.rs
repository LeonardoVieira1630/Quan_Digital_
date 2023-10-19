// get_candles.rs - Candle Functions

// This file provides functions to interact with the Binance API and get candle information for a specified trading pair.

// The get_candles.rs file provides convenient functions to fetch candle data from the Binance API, which is essential for
// performing technical analysis, backtesting strategies, or generating trading operations.

// Ensure that you have a valid Binance API key and secret key to authenticate the API requests. It is crucial to handle
// API authentication securely, following best practices, and protect sensitive information.

// Please refer to the Binance API documentation for detailed information on the required parameters, possible responses,
// and any additional considerations for retrieving candle data.

#![allow(unused_variables)]
use crate::models::KlineData;
use async_recursion::async_recursion;
use chrono::prelude::*;
const ONE_MIN_IN_MILLISECONDS: u64 = 60000;
use std::collections::hash_map;
use std::io::Error;
use std::time::{Duration, Instant};
static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));
use crate::error;
use crate::{binance_orders, re_send_request};
use binance_orders::{exchange_url, get_client, get_signature, get_timestamp};
use error::*;
use hmac::{Hmac, Mac, NewMac};
use reqwest::{header, StatusCode};
use sha2::Sha256;
use std::collections::{BTreeMap, HashMap};
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};

/// Get the last one minute closed candle's price for the BTCUSDT trading pair.
///
/// This function retrieves the last closed candle's price for the BTCUSDT trading pair with a 1-minute interval.
///
/// # Returns
///
/// - `Ok(f64)`: The last closed candle's price, greater than 0.0.
/// - `Err(String)`: An error message if the request fails.
///
#[async_recursion]
pub async fn get_candle_last_min() -> Result<f64, String> {
    let time_now = Utc::now().timestamp_millis() as u64;
    let start_time = time_now - 2 * ONE_MIN_IN_MILLISECONDS;

    let client: reqwest::Client = get_client().await;
    let timestamp = get_timestamp(SystemTime::now()).await;
    let params = format!(
        "symbol=BTCUSDT&interval=1m&startTime={}&endTime={}",
        start_time, time_now
    );
    let signature = get_signature(params.clone()).await;

    let request = format!(
        "{}/fapi/v1/klines?{}&signature={}",
        exchange_url().await,
        params.clone(),
        signature.clone()
    );

    let result = match client.get(request.clone()).send().await {
        Ok(response) => response,
        Err(_) => re_send_request(client, request, "GET").await,
    };
    if result.status() == StatusCode::OK {
        let data: Vec<KlineData> = result.json().await.unwrap();
        let price_data: Vec<f64> = data.iter().rev().take(2).map(|f| f.close).collect();
        let last_closed_price: f64 = price_data[1];
        Ok(last_closed_price)
    } else {
        let error = error_handler(result, None).await;
        if error == "E03: Error 502, exchange server is in trouble." {
            Err(error)
        } else if error == "E07: Dns error: No such host is known."
            || error == "E08: Timestamp for this request is outside of the recvWindow"
        {
            get_candle_last_min().await
        } else {
            println!("{}", error);
            std::process::exit(1);
        }
    }
}

/// Get the closing prices of the last 'quantity' one-minute candles for the BTCUSDT trading pair.
///
/// This function retrieves the closing prices of the last 'quantity' one-minute candles for the BTCUSDT trading pair.
///
/// # Arguments
///
/// - `quantity`: The number of one-minute candles to retrieve.
///
/// # Returns
///
/// - `Ok(BTreeMap<i64, f64>)`: A `BTreeMap` where the key is the timestamp and the value is the closing price.
/// - `Err(String)`: An error message if the request fails.
///
#[async_recursion]
pub async fn get_some_1m_candle(quantity: i64) -> Result<BTreeMap<i64, f64>, String> {
    let time_now = Utc::now().timestamp_millis() as u64;
    let start_time = time_now - ((quantity + 1) as u64) * ONE_MIN_IN_MILLISECONDS;

    let client: reqwest::Client = get_client().await;
    let timestamp = get_timestamp(SystemTime::now()).await;
    let params = format!(
        "symbol=BTCUSDT&interval=1m&startTime={}&endTime={}",
        start_time, time_now
    );
    let signature = get_signature(params.clone()).await;

    let request = format!(
        "{}/fapi/v1/klines?{}&signature={}",
        exchange_url().await,
        params.clone(),
        signature.clone()
    );

    let result = match client.get(request.clone()).send().await {
        Ok(response) => response,
        Err(_) => re_send_request(client, request, "GET").await,
    };
    if result.status() == StatusCode::OK {
        let data: Vec<KlineData> = result.json().await.unwrap();
        let price_data: Vec<f64> = data
            .iter()
            .take(quantity as usize)
            .map(|f| f.close)
            .collect();

        let date_data: Vec<i64> = data
            .iter()
            .take(quantity as usize)
            .map(|f| f.open_time)
            .collect();

        //let mut info_data: HashMap::new();
        let mut info_data: BTreeMap<i64, f64> = BTreeMap::new();
        let mut i = 0;
        while i < price_data.len() {
            info_data.insert(date_data[i], price_data[i]);
            i += 1;
        }

        Ok(info_data)
    } else {
        let error = error_handler(result, None).await;
        if error == "E03: Error 502, exchange server is in trouble." {
            Err(error)
        } else if error == "E07: Dns error: No such host is known."
            || error == "E08: Timestamp for this request is outside of the recvWindow"
        {
            get_some_1m_candle(quantity).await
        } else {
            println!("{}", error);
            std::process::exit(1);
        }
    }
}

/// Get the closing prices of 'quantity' candles for a specified trading pair and interval.
///
/// This function retrieves the closing prices of 'quantity' candles for a specified trading pair and interval.
///
/// # Arguments
///
/// - `quantity`: The number of candles to retrieve.
/// - `symbol`: The trading pair symbol (e.g., "BTCUSDT").
/// - `interval`: The candle interval as a string (e.g., "1h").
///
/// # Returns
///
/// - `Ok(Vec<f64>)`: A vector containing the closing prices of the retrieved candles.
/// - `Err(String)`: An error message if the request fails.
///
pub async fn get_candle_info(
    quantity: usize,
    symbol: &str,
    interval: String,
) -> Result<Vec<f64>, String> {
    //Split interval string into period (m) and candle length (15)
    let period: char = interval.chars().last().unwrap();
    let mut candle_length = interval;
    candle_length.pop().unwrap();

    //calculating how many on minute candles will be needed
    let one_min_quantity: i64;
    if period == 'm' {
        one_min_quantity = (quantity) as i64 * candle_length.parse::<i64>().unwrap();
    } else if period == 'h' {
        one_min_quantity = (quantity) as i64 * 60 * candle_length.parse::<i64>().unwrap();
    // } else if period == 'd' {
    //     one_min_quantity = (quantity as i64 + 2) * 60 * 24 * candle_length.parse::<i64>().unwrap();
    } else {
        //if the interval is not valid, the number of candles requested will be "quantity".
        panic!("get_candle_info: Interval not implemented.");
    }

    // Getting exchange candles
    let candle_1m_result = get_some_1m_candle(one_min_quantity).await;

    if let Ok(candle_1m) = candle_1m_result {
        //Building requested candles
        let mut candles: Vec<f64> = Vec::new();
        let mut i = 0;
        //let mut temp: f64 = 0.0;
        let mut is_opened = false;
        for (date, price) in candle_1m {
            //New candle opening
            let data_in_seconds = date / 1000;
            if data_in_seconds % ((one_min_quantity / (quantity) as i64) * 60) == 0 {
                candles.push(price);
                i += 1;
                //temp = price;
                if !is_opened {
                    is_opened = true
                }
            }
            //Updating last candle opened
            else if is_opened {
                candles[i - 1] = price;
                //temp = price;
            }
        }

        Ok(candles)
    } else {
        // Handle the error from retrieving the 1-hour candle data
        eprintln!("Failed to retrieve candles: {:?}", candle_1m_result);
        Err("Failed to retrieve candles".to_string())
    }
}

/// Get the closing prices of 'quantity' candles for a specified trading pair and interval from Binance (
/// interval needs to be a binance one).
///
/// This function retrieves the closing prices of 'quantity' candles for a specified trading pair and interval from Binance.
///
/// # Arguments
///
/// - `quantity`: The number of candles to retrieve.
/// - `interval`: The candle interval as a string (e.g., "1h").
///
/// # Returns
///
/// - `Ok(BTreeMap<i64, f64>)`: A `BTreeMap` where the key is the timestamp and the value is the closing price.
/// - `Err(String)`: An error message if the request fails.
///
#[async_recursion]
pub async fn get_some_candles_from_binance(
    quantity: i64,
    interval: &str,
) -> Result<BTreeMap<i64, f64>, String> {
    //Split interval string into period (m) and candle length (15)
    let period: char = interval.chars().last().unwrap();
    let mut candle_length = interval.to_string();
    candle_length.pop().unwrap();

    //calculating how many on minute candles will be needed
    let one_min_quantity: i64;
    if period == 'm' {
        one_min_quantity = (quantity + 1) * candle_length.parse::<i64>().unwrap();
    } else if period == 'h' {
        one_min_quantity = (quantity + 1) * 60 * candle_length.parse::<i64>().unwrap();
    } else if period == 'd' {
        one_min_quantity = (quantity + 1) * 60 * 24 * candle_length.parse::<i64>().unwrap();
    } else {
        //if the interval is not valid, the number of candles requested will be "quantity".
        panic!("get_candle_info: Interval not implemented.");
    }

    let time_now = Utc::now().timestamp_millis() as u64;
    let start_time = time_now - ((one_min_quantity + 1) as u64) * ONE_MIN_IN_MILLISECONDS;

    let client: reqwest::Client = get_client().await;
    let timestamp = get_timestamp(SystemTime::now()).await;

    let params = format!(
        "symbol=BTCUSDT&interval={}&startTime={}&endTime={}",
        interval, start_time, time_now
    );

    let signature = get_signature(params.clone()).await;

    let request = format!(
        "{}/fapi/v1/klines?{}&signature={}",
        exchange_url().await,
        params.clone(),
        signature.clone()
    );

    let result = match client.get(request.clone()).send().await {
        Ok(response) => response,
        Err(_) => re_send_request(client, request, "GET").await,
    };
    if result.status() == StatusCode::OK {
        let data: Vec<KlineData> = result.json().await.unwrap();
        let price_data: Vec<f64> = data
            .iter()
            .take(quantity as usize)
            .map(|f| f.close)
            .collect();

        let date_data: Vec<i64> = data
            .iter()
            .take(quantity as usize)
            .map(|f| f.open_time)
            .collect();

        //let mut info_data: HashMap::new();
        let mut info_data: BTreeMap<i64, f64> = BTreeMap::new();
        let mut i = 0;
        while i < price_data.len() {
            info_data.insert(date_data[i], price_data[i]);
            i += 1;
        }
        Ok(info_data)
    } else {
        let error = error_handler(result, None).await;
        if error == "E03: Error 502, exchange server is in trouble." {
            Err(error)
        } else if error == "E07: Dns error: No such host is known."
            || error == "E08: Timestamp for this request is outside of the recvWindow"
        {
            get_some_candles_from_binance(quantity, interval).await
        } else {
            println!("{}", error);
            std::process::exit(1);
        }
    }
}

/// Build candles with closing prices from one-hour candles for a specified quantity and interval.
///
/// This function builds candles with closing prices from one-hour candles for a specified quantity and interval.
///
/// # Arguments
///
/// - `quantity`: The number of candles to build.
/// - `symbol`: The trading pair symbol (e.g., "BTCUSDT").
/// - `interval`: The candle interval as a string (e.g., "3h").
///
/// # Returns
///
/// - `Ok(Vec<f64>)`: A vector containing the closing prices of the built candles.
/// - `Err(String)`: An error message if the request fails.
///
pub async fn build_candle_w_1hr_close_price(
    quantity: usize,
    symbol: &str,
    interval: String,
) -> Result<Vec<f64>, String> {
    //Split interval string into period (m) and candle length (15)
    let period: char = interval.chars().last().unwrap();
    let mut candle_length = interval;
    candle_length.pop().unwrap();

    //calculating how many on minute candles will be needed
    let one_min_quantity: i64;
    if period == 'h' {
        one_min_quantity = (quantity as i64) * candle_length.parse::<i64>().unwrap();
    } else if period == 'd' {
        one_min_quantity = (quantity as i64) * 24 * candle_length.parse::<i64>().unwrap();
    } else {
        //if the interval is not valid, the number of candles requested will be "quantity".
        panic!("get_candle_info: Interval not implemented.");
    }

    // Getting exchange candles
    let candle_1m_result = get_some_1hr_candle(one_min_quantity).await;

    let candle_1m: BTreeMap<i64, f64>;
    match candle_1m_result {
        Ok(map) => {
            let candle_1m = map;

            //Building requested candles
            let mut candles: Vec<f64> = Vec::new();
            let mut i = 0;
            //let mut temp: f64 = 0.0;
            let mut is_opened = false;
            for (date, price) in candle_1m {
                //New candle opening
                let data_in_seconds = date / 1000;
                if data_in_seconds % (candle_length.parse::<i64>().unwrap() * 60 * 60) == 0 {
                    candles.push(price);
                    i += 1;
                    //temp = price;
                    if !is_opened {
                        is_opened = true
                    }
                }
                //Updating last candle opened
                else if is_opened {
                    candles[i - 1] = price;
                    //temp = price;
                }
            }

            Ok(candles)
        }
        Err(err) => Err(err),
    }
}

/// Get the closing prices of the last 'quantity' one-hour candles for the BTCUSDT trading pair.
///
/// This function retrieves the closing prices of the last 'quantity' one-hour candles for the BTCUSDT trading pair.
///
/// # Arguments
///
/// - `quantity`: The number of one-hour candles to retrieve.
///
/// # Returns
///
/// - `Ok(BTreeMap<i64, f64>)`: A `BTreeMap` where the key is the timestamp and the value is the closing price.
/// - `Err(String)`: An error message if the request fails.
///
#[async_recursion]
pub async fn get_some_1hr_candle(quantity: i64) -> Result<BTreeMap<i64, f64>, String> {
    let time_now = Utc::now().timestamp_millis() as u64;
    let start_time = time_now - ((quantity * 60) as u64) * ONE_MIN_IN_MILLISECONDS;

    let client: reqwest::Client = get_client().await;
    let timestamp = get_timestamp(SystemTime::now()).await;
    let params = format!(
        "symbol=BTCUSDT&interval=1h&startTime={}&endTime={}",
        start_time, time_now
    );
    let signature = get_signature(params.clone()).await;

    let request = format!(
        "{}/fapi/v1/klines?{}&signature={}",
        exchange_url().await,
        params.clone(),
        signature.clone()
    );

    let result = match client.get(request.clone()).send().await {
        Ok(response) => response,
        Err(_) => re_send_request(client, request, "GET").await,
    };
    if result.status() == StatusCode::OK {
        let data: Vec<KlineData> = result.json().await.unwrap();
        let price_data: Vec<f64> = data
            .iter()
            .take(quantity as usize)
            .map(|f| f.close)
            .collect();
        //price_data.pop();

        let date_data: Vec<i64> = data
            .iter()
            .take(quantity as usize)
            .map(|f| f.open_time)
            .collect();
        //date_data.pop();

        //let mut info_data: HashMap::new();
        let mut info_data: BTreeMap<i64, f64> = BTreeMap::new();
        let mut i = 0;
        while i < price_data.len() {
            info_data.insert(date_data[i], price_data[i]);
            i += 1;
        }

        Ok(info_data)
    } else {
        let error = error_handler(result, None).await;
        if error == "E03: Error 502, exchange server is in trouble." {
            Err(error)
        } else if error == "E07: Dns error: No such host is known."
            || error == "E08: Timestamp for this request is outside of the recvWindow"
        {
            get_some_1hr_candle(quantity).await
        } else {
            println!("{}", error);
            std::process::exit(1);
        }
    }
}

//Functions tests
#[cfg(test)]
mod tests {
    use super::*;
    use tokio::test;

    /// Test function for the `get_candle_last_min` function.
    ///
    /// This test verifies that the `get_candle_last_min` function returns a valid result.
    /// It checks whether the result is `Ok`, and the unwrapped value is greater than 0.0.
    ///
    #[test]
    async fn get_candle_last_min_test() {
        let res = get_candle_last_min().await;
        assert!(res.is_ok());
        let res_unwrapped = res.unwrap();
        assert!(res_unwrapped > 0.0);
    }

    /// Test function for the `get_some_1m_candle` function.
    ///
    /// This test verifies that the `get_some_1m_candle` function returns a valid result.
    /// It checks whether the result is `Ok`, and the unwrapped value is greater than 0.0.
    /// It also ensures that the retrieved data has the correct length and that timestamps are ordered.
    ///
    #[test]
    async fn get_some_1m_candle_test() {
        let res = get_some_1m_candle(10).await;
        assert!(res.is_ok());

        let res_unwrapped = res.unwrap();
        let mut previous_time: Option<i64> = None;

        //Asserting we received the correct length
        assert!(res_unwrapped.len() == 10);

        for (time, value) in res_unwrapped {
            //Asserting that the value makes sense
            assert!(value > 0.0);

            //Asserting that the values are ordinate in time
            if let Some(prev_time) = previous_time {
                assert!(time == (prev_time + 60_000))
            }
            previous_time = Some(time);
        }
    }

    /// Test function for the `get_candle_info` function.
    ///
    /// This test verifies that the `get_candle_info` function returns a valid result.
    /// It checks whether the result is `Ok` and ensures that the retrieved data has the correct length
    /// and that all values are greater than 0.0.
    ///
    #[test]
    async fn get_candle_info_test() {
        let res = get_candle_info(7, "BTCUSDT", "30m".to_string()).await;
        assert!(res.is_ok());

        let res_unwrapped = res.unwrap();

        //Asserting we received the correct length
        assert!(res_unwrapped.len() == 7);

        for value in res_unwrapped {
            //Asserting that the value makes sense
            assert!(value > 0.0);
        }
    }

    /// Test function for the `get_candle_info` function with a 1-hour interval.
    ///
    /// This test verifies that the `get_candle_info` function with a 1-hour interval returns a valid result.
    /// It checks whether the result is `Ok` and ensures that the retrieved data has the correct length
    /// and that all values are greater than 0.0.
    ///
    #[test]
    async fn get_candle_info_hours_test() {
        let res = get_candle_info(7, "BTCUSDT", "1h".to_string()).await;
        assert!(res.is_ok());

        let res_unwrapped = res.unwrap();

        //Asserting we received the correct length
        assert!(res_unwrapped.len() == 7);

        for value in res_unwrapped {
            //Asserting that the value makes sense
            assert!(value > 0.0);
        }
    }

    /// Test function for the `get_some_candles_from_binance` function with a 1-hour interval.
    ///
    /// This test verifies that the `get_some_candles_from_binance` function with a 1-hour interval
    /// returns a valid result. It checks whether the result is `Ok`, ensures that the retrieved data
    /// has the correct length, and that timestamps are ordered correctly.
    ///
    #[test]
    async fn get_some_candles_from_binance_hours_test() {
        let res = get_some_candles_from_binance(7, "1h").await;
        assert!(res.is_ok());

        let res_unwrapped = res.unwrap();
        let mut previous_time: Option<i64> = None;

        //Asserting we received the correct length
        assert!(res_unwrapped.len() == 7);

        for (time, value) in res_unwrapped {
            //Asserting that the value makes sense
            //Asserting that the values are ordinate in time
            if let Some(prev_time) = previous_time {
                assert!(time == (prev_time + 1000 * 3600))
            }
            previous_time = Some(time);
        }
    }

    /// Test function for the `get_some_candles_from_binance` function with a 30-minute interval.
    ///
    /// This test verifies that the `get_some_candles_from_binance` function with a 30-minute interval
    /// returns a valid result. It checks whether the result is `Ok`, ensures that the retrieved data
    /// has the correct length, that values are greater than 0.0, and that timestamps are ordered correctly.
    ///
    #[test]
    async fn get_some_candles_from_binance_minutes_test() {
        let res = get_some_candles_from_binance(7, "30m").await;
        assert!(res.is_ok());

        let res_unwrapped = res.unwrap();
        let mut previous_time: Option<i64> = None;

        //Asserting we received the correct length
        assert!(res_unwrapped.len() == 7);

        for (time, value) in res_unwrapped {
            //Asserting that the value makes sense
            assert!(value > 0.0);
            //Asserting that the values are ordinate in time
            if let Some(prev_time) = previous_time {
                assert!(time == (prev_time + 1000 * 60 * 30))
            }
            previous_time = Some(time);
        }
    }
}
