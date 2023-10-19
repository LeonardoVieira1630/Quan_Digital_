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
use crate::*;
use binance_orders::{exchange_url, get_client, get_signature, get_timestamp};
use error::*;
use hmac::{Hmac, Mac, NewMac};
use reqwest::{header, StatusCode};
use sha2::Sha256;
use std::collections::{BTreeMap, HashMap};
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};

/// Get the lowest price of the most recent closed 1-minute candle.
///
/// This function fetches the 1-minute candle data for the most recent closed candle and returns the lowest price recorded during that candle.
///
/// # Returns
///
/// - `Ok(f64)`: The lowest price of the last closed 1-minute candle.
/// - `Err(String)`: An error message if the request fails or encounters an issue.
///
#[async_recursion]
pub async fn get_candle_last_min_min_value() -> Result<f64, String> {
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
        let price_data: Vec<f64> = data.iter().take(1).map(|f| f.low).collect();
        let last_closed_price: f64 = price_data[0];
        Ok(last_closed_price)
    } else {
        let error = error_handler(result, None).await;
        if error == "E03: Error 502, exchange server is in trouble." {
            Err(error)
        } else if error == "E07: Dns error: No such host is known."
            || error == "E08: Timestamp for this request is outside of the recvWindow"
        {
            get_candle_last_min_min_value().await
        } else {
            println!("{}", error);
            std::process::exit(1);
        }
    }
}

/// Get the lowest prices of a specified number of 1-minute candles.
///
/// This function fetches the 1-minute candle data for the specified number of candles and returns a `BTreeMap` with timestamps as keys and lowest prices as values.
///
/// # Arguments
///
/// - `quantity`: The number of 1-minute candles to retrieve.
///
/// # Returns
///
/// - `Ok(BTreeMap<i64, f64>)`: A `BTreeMap` where timestamps are keys, and the lowest prices are values.
/// - `Err(String)`: An error message if the request fails or encounters an issue.
///
#[async_recursion]
pub async fn get_some_1m_candle_min_value(quantity: i64) -> Result<BTreeMap<i64, f64>, String> {
    let time_now = Utc::now().timestamp_millis() as u64;
    let start_time = time_now - ((quantity + 1) as u64) * ONE_MIN_IN_MILLISECONDS;

    let client: reqwest::Client = get_client().await;
    let timestamp = get_timestamp(SystemTime::now()).await;
    let params = format!(
        "symbol=BTCUSDT&interval=1m&startTime={}&endTime={}&limit=1500",
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
        Ok(response) => {
            response
            // if verify_response_body(response).await.is_ok() {
            //     response
            // } else {
            //     re_send_request(client, request, "GET").await
            // }
        }
        Err(_) => re_send_request(client, request, "GET").await,
    };
    if result.status() == StatusCode::OK {
        let data: Vec<KlineData> = result.json().await.unwrap();
        println!("data len: {}", data.len());
        let price_data: Vec<f64> = data.iter().take(quantity as usize).map(|f| f.low).collect();
        println!("price_data len: {}", price_data.len());

        let date_data: Vec<i64> = data
            .iter()
            .take(quantity as usize)
            .map(|f| f.open_time)
            .collect();
        println!("date_data len: {}", date_data.len());

        //let mut info_data: HashMap::new();
        let mut info_data: BTreeMap<i64, f64> = BTreeMap::new();
        let mut i = 0;
        while i < price_data.len() {
            info_data.insert(date_data[i], price_data[i]);
            //println!("inserting: {}", date_data[i]);

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
            get_some_1m_candle_min_value(quantity).await
        } else {
            println!("{}", error);
            std::process::exit(1);
        }
    }
}

/// Get the lowest prices of a specified number of candles for a given symbol and interval.
///
/// This function fetches the candle data for the specified number of candles and returns a `Vec` of lowest prices.
///
/// # Arguments
///
/// - `quantity`: The number of candles to retrieve.
/// - `symbol`: The trading pair symbol (e.g., "BTCUSDT").
/// - `interval`: The candle interval in the format "Xm" or "Xh" (e.g., "1m", "1h").
///
/// # Returns
///
/// - `Ok(Vec<f64>)`: A `Vec` containing the lowest prices of the specified candles.
/// - `Err(String)`: An error message if the request fails or encounters an issue.
///
pub async fn get_candle_info_min_value(
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
        one_min_quantity = ((quantity + 2) as i64) * candle_length.parse::<i64>().unwrap();
    } else if period == 'h' {
        one_min_quantity = ((quantity + 2) as i64) * 60 * candle_length.parse::<i64>().unwrap();
    // } else if period == 'd' {
    //     one_min_quantity =
    //         ((quantity + 1) as i64) * 60 * 24 * candle_length.parse::<i64>().unwrap();
    } else {
        //if the interval is not valid, the number of candles requested will be "quantity".
        panic!("get_candle_info: Interval not implemented.");
    }
    println!("quantity: {}", one_min_quantity);

    // Getting exchange candles
    let candle_1m_result = get_some_1m_candle_min_value(one_min_quantity).await;

    //let candle_1m: BTreeMap<i64, f64>;
    if let Ok(candle_1m) = candle_1m_result {
        // for (key, value) in candle_1m.clone() {
        //     println!(" print cand: {} {}", key, value);
        // }
        //println!("len: {}", candle_1m.keys().len());

        // Define the desired time frame
        let candle_length = candle_length.parse::<i64>().unwrap();

        // Building requested candles
        let mut candles: Vec<f64> = Vec::new();
        let mut min_value: f64 = 0.0;
        let i = 0;
        let mut is_opened = false;

        for (date, price) in candle_1m {
            // New candle opening
            let data_in_seconds = date / 1000;
            if data_in_seconds % ((one_min_quantity / (quantity + 2) as i64) * 60) == 0 {
                if is_opened {
                    candles.push(min_value);
                    min_value = f64::MAX;
                }
                is_opened = true;
            }

            // Track the maximum value
            if price < min_value {
                min_value = price;
            }
            //println!("{} {}", date, price);
        }

        // Add the last max value to the candles if necessary
        if is_opened {
            candles.push(min_value);
        }

        //println!("yes body {}", candles.len());

        // for candle in candles.clone() {
        //     println!(" print cand: {}", candle);
        // }

        candles.remove(0);
        candles.pop();

        Ok(candles)
    } else {
        // Handle the error from retrieving the 1-hour candle data
        eprintln!("Failed to retrieve  candles: {:?}", candle_1m_result);
        Err("Failed to retrieve 1-hour candles".to_string())
    }
}

/// Get the lowest prices of a specified number of candles from Binance for a given interval. (Interval needs to
/// be an binance one).
///
/// This function fetches the candle data from Binance for the specified number of candles and returns a `BTreeMap` with timestamps as keys and lowest prices as values.
///
/// # Arguments
///
/// - `quantity`: The number of candles to retrieve.
/// - `interval`: The candle interval in the format "Xm" or "Xh" (e.g., "30m", "1h").
///
/// # Returns
///
/// - `Ok(BTreeMap<i64, f64>)`: A `BTreeMap` where timestamps are keys, and the lowest prices are values.
/// - `Err(String)`: An error message if the request fails or encounters an issue.
///
#[async_recursion]
pub async fn get_some_candles_from_binance_min_value(
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
    let start_time = time_now - ((one_min_quantity) as u64) * ONE_MIN_IN_MILLISECONDS;

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
        let price_data: Vec<f64> = data.iter().take(quantity as usize).map(|f| f.low).collect();

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
            get_some_candles_from_binance_min_value(quantity, interval).await
        } else {
            println!("{}", error);
            std::process::exit(1);
        }
    }
}

/// Get the lowest prices of a specified number of 1-hour candles.
///
/// This function fetches the 1-hour candle data for the specified number of candles and returns a `BTreeMap` with timestamps as keys and lowest prices as values.
///
/// # Arguments
///
/// - `quantity`: The number of 1-hour candles to retrieve.
///
/// # Returns
///
/// - `Ok(BTreeMap<i64, f64>)`: A `BTreeMap` where timestamps are keys, and the lowest prices are values.
/// - `Err(String)`: An error message if the request fails or encounters an issue.
///
#[async_recursion]
pub async fn get_some_1hr_candle_min_value(quantity: i64) -> Result<BTreeMap<i64, f64>, String> {
    let time_now = Utc::now().timestamp_millis() as u64;
    let start_time = time_now - ((quantity) as u64) * ONE_MIN_IN_MILLISECONDS;

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
        let price_data: Vec<f64> = data.iter().take(quantity as usize).map(|f| f.low).collect();
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
            get_some_1hr_candle_min_value(quantity).await
        } else {
            println!("{}", error);
            std::process::exit(1);
        }
    }
}

/// Build a series of lowest prices using 1hr candles for a custom interval.
///
/// This function builds a series of lowest prices based on the specified interval and quantity of candles.
///
/// # Arguments
///
/// - `quantity`: The number of candles to build.
/// - `symbol`: The trading pair symbol (e.g., "BTCUSDT").
/// - `interval`: The custom candle interval in the format "Xh" (e.g., "3h").
///
/// # Returns
///
/// - `Ok(Vec<f64>)`: A `Vec` containing the lowest prices of the specified candles.
/// - `Err(String)`: An error message if the request fails or encounters an issue.
///
pub async fn build_candle_w_1hr_min_price(
    quantity: usize,
    symbol: &str,
    interval: String,
) -> Result<Vec<f64>, String> {
    // Split interval string into period (m) and candle length (15)
    let period: char = interval.chars().last().unwrap();
    let mut candle_length = interval;
    candle_length.pop().unwrap();

    // Calculating how many one-minute candles will be needed
    let one_min_quantity: i64;
    if period == 'h' {
        one_min_quantity = ((quantity + 2) as i64) * 60 * candle_length.parse::<i64>().unwrap();
    } else if period == 'd' {
        one_min_quantity =
            ((quantity + 1) as i64) * 60 * 24 * candle_length.parse::<i64>().unwrap();
    } else {
        panic!("build_candle_w_1hr_min_price: Interval not implemented.");
    }

    // Getting exchange candles
    let candle_1m_result = get_some_1hr_candle_min_value(one_min_quantity).await;

    if let Ok(candle_1m) = candle_1m_result {
        // Building requested candles
        let mut candles: Vec<f64> = Vec::new();
        let mut min_value: f64 = f64::MAX;
        let i = 0;
        let mut is_opened = false;

        for (date, price) in candle_1m {
            // New candle opening
            let data_in_seconds = date / 1000;
            if data_in_seconds % ((one_min_quantity / (quantity + 2) as i64) * 60) == 0 {
                if is_opened {
                    candles.push(min_value);
                    min_value = f64::MAX;
                }
                is_opened = true;
            }

            // Track the minimum value
            if price < min_value {
                min_value = price;
            }
        }

        //candles.remove(0);

        // Add the last min value to the candles if necessary
        if is_opened {
            candles.push(min_value);
        }
        candles.pop();
        candles.remove(0);

        Ok(candles)
    } else {
        // Handle the error from retrieving the 1-hour candle data
        eprintln!("Failed to retrieve 1-hour candles: {:?}", candle_1m_result);
        Err("Failed to retrieve 1-hour candles".to_string())
    }
}

/// Get the lowest price among a specified number of candles from Binance for a given interval (Interval needs to
/// be an binance one).
///
/// This function fetches the candle data from Binance for the specified number of candles and returns the lowest price among them.
///
/// # Arguments
///
/// - `quantity`: The number of candles to retrieve.
/// - `interval`: The candle interval in the format "Xm" or "Xh" (e.g., "30m", "1h").
///
/// # Returns
///
/// - `f64`: The lowest price among the specified candles.
///
pub async fn get_lowest_candle_from_binance_candles(quantity: i64, interval: &str) -> f64 {
    let data = get_some_candles_from_binance_min_value(quantity, interval)
        .await
        .unwrap();

    let mut min_price = f64::MAX;
    for (date, close_price) in data {
        if min_price > close_price {
            min_price = close_price;
        }
    }
    min_price
}

/// Get the lowest price among a specified number of candles for a given symbol and interval.
///
/// This function fetches the candle data for the specified number of candles and returns the lowest price among them.
///
/// # Arguments
///
/// - `quantity`: The number of candles to retrieve.
/// - `interval`: The candle interval in the format "Xm" or "Xh" (e.g., "1h", "4h").
///
/// # Returns
///
/// - `f64`: The lowest price among the specified candles.
///
pub async fn get_lowest_candle(quantity: i64, interval: &str) -> f64 {
    let data = get_candle_info_min_value(quantity as usize, "BTCUSDT", interval.to_string())
        .await
        .unwrap();

    let mut min_price = f64::MAX;
    for close_price in data {
        if min_price > close_price {
            min_price = close_price;
        }
    }
    min_price
}

//Functions tests
#[cfg(test)]
mod tests {
    use super::*;
    use tokio::test;

    /// Test for the `get_candle_last_minute_min_value` function.
    ///
    /// This test verifies that the function successfully retrieves the lowest price of the most recent closed 1-minute candle.
    ///
    /// The test asserts that:
    /// - The result is successful (Ok).
    /// - The lowest price obtained is greater than 0.0.
    ///
    #[test]
    async fn get_candle_last_minute_min_value_test() {
        let res = get_candle_last_min_min_value().await;
        assert!(res.is_ok());

        let res_unwrapped = res.unwrap();
        assert!(res_unwrapped > 0.0);
    }

    /// Test for the `get_some_1m_candle_min_value` function.
    ///
    /// This test verifies that the function successfully retrieves a specified number of 1-minute candles and checks their properties.
    ///
    /// The test asserts that:
    /// - The result is successful (Ok).
    /// - The number of retrieved candles matches the specified quantity.
    /// - The lowest prices are greater than 0.0.
    /// - The timestamps of the retrieved candles are in ascending order.
    ///
    #[test]
    async fn get_some_1m_candle_min_value_test() {
        let res = get_some_1m_candle_min_value(10).await;
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

    /// Test for the `get_candle_info_min_value` function with minute interval.
    ///
    /// This test verifies that the function successfully retrieves a specified number of minute candles for a given symbol and interval and checks their properties.
    ///
    /// The test asserts that:
    /// - The result is successful (Ok).
    /// - The number of retrieved candles matches the specified quantity.
    /// - The lowest prices are greater than 0.0.
    ///
    #[test]
    async fn get_candle_info_min_value_minutes_test() {
        let res = get_candle_info_min_value(7, "BTCUSDT", "6m".to_string()).await;
        assert!(res.is_ok());

        let res_unwrapped = res.unwrap();

        //Asserting we received the correct length
        assert!(res_unwrapped.len() == 7);

        for value in res_unwrapped {
            //Asserting that the value makes sense
            assert!(value > 0.0);
        }
    }

    /// Test for the `get_candle_info_min_value` function with hour interval.
    ///
    /// This test verifies that the function successfully retrieves a specified number of hour candles for a given symbol and interval and checks their properties.
    ///
    /// The test asserts that:
    /// - The result is successful (Ok).
    /// - The number of retrieved candles matches the specified quantity.
    /// - The lowest prices are greater than 0.0.
    ///
    #[test]
    async fn get_candle_info_min_value_hours_test() {
        let res = get_candle_info_min_value(7, "BTCUSDT", "1h".to_string()).await;
        assert!(res.is_ok());

        let res_unwrapped = res.unwrap();

        //Asserting we received the correct length
        assert!(res_unwrapped.len() == 7);

        for value in res_unwrapped {
            //Asserting that the value makes sense
            assert!(value > 0.0);
        }
    }

    /// Test for the `get_some_candles_from_binance_min_value` function with hour interval.
    ///
    /// This test verifies that the function successfully retrieves a specified number of hour candles from Binance for a given interval and checks their properties.
    ///
    /// The test asserts that:
    /// - The result is successful (Ok).
    /// - The number of retrieved candles matches the specified quantity.
    /// - The timestamps of the retrieved candles are in ascending order.
    ///
    #[test]
    async fn get_some_candles_from_binance_min_value_hours_test() {
        let res = get_some_candles_from_binance_min_value(7, "1h").await;
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

    /// Test for the `get_some_candles_from_binance_min_value` function with minute interval.
    ///
    /// This test verifies that the function successfully retrieves a specified number of minute candles from Binance for a given interval and checks their properties.
    ///
    /// The test asserts that:
    /// - The result is successful (Ok).
    /// - The number of retrieved candles matches the specified quantity.
    /// - The lowest prices are greater than 0.0.
    /// - The timestamps of the retrieved candles are in ascending order.
    ///
    #[test]
    async fn get_some_candles_from_binance_min_value_minutes_test() {
        let res = get_some_candles_from_binance_min_value(7, "30m").await;
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

    /// Test for the `build_candle_w_1hr_min_price` function.
    ///
    /// This test verifies that the function successfully builds a series of lowest prices for a custom interval and checks their properties.
    ///
    /// The test asserts that:
    /// - The result is successful (Ok).
    /// - The number of generated prices matches the specified quantity.
    /// - The lowest prices are greater than 0.0.
    ///
    #[test]
    async fn test_build_candle_w_1hr_min_price() {
        // Chame a função que você está testando
        let result = build_candle_w_1hr_min_price(16, "BTCUSDT", "3h".to_string()).await;

        // Verifique se a função retornou Ok
        assert!(result.is_ok());

        // Desembrulhe o resultado Ok para obter o vetor de preços
        let prices = result.unwrap();

        // Verifique se o tamanho do vetor de preços é o esperado (3 no exemplo)
        assert_eq!(prices.len(), 16);

        for value in prices {
            //Asserting that the value makes sense
            assert!(value > 0.0);
            //println!("{}", value);
        }
    }

    /// Test for the `get_lowest_candle_from_binance_candles` function.
    ///
    /// This test verifies that the function successfully retrieves the lowest price among a specified number of candles from Binance for a given interval.
    ///
    /// The test asserts that:
    /// - The retrieved lowest price is greater than 0.0.
    ///
    #[test]
    async fn get_lowest_candle_from_binance_candles_test() {
        let res: f64 = get_lowest_candle_from_binance_candles(2, "3m").await;
        assert!(res > 0.0);
    }

    /// Test for the `get_lowest_candle` function.
    ///
    /// This test verifies that the function successfully retrieves the lowest price among a specified number of candles for a given interval.
    ///
    /// The test asserts that:
    /// - The retrieved lowest price is greater than 0.0.
    ///
    #[test]
    async fn get_lowest_candle_building_candles_test() {
        let res: f64 = get_lowest_candle(15, "6m").await;
        println!("{}", res);
        assert!(res > 0.0);
    }
}
