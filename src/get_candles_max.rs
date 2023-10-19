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
use crate::binance_orders::*;
use crate::error;
use error::*;
use hmac::{Hmac, Mac, NewMac};
use reqwest::{header, StatusCode};
use sha2::Sha256;
use std::collections::{BTreeMap, HashMap};
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};

/// Get the maximum value from the last one-minute closed candle.
///
/// This function retrieves the last one-minute closed candle for the specified trading pair (e.g., BTCUSDT)
/// and returns the maximum (high) value from that candle.
///
/// Returns:
/// - `Ok(f64)`: The maximum value from the last closed candle.
/// - `Err(String)`: An error message if the request fails.
///
#[async_recursion]
pub async fn get_candle_last_minute_max_value() -> Result<f64, String> {
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

        let price_data: Vec<f64> = data.iter().take(1).map(|f| f.high).collect();
        let last_closed_price: f64 = price_data[0];
        Ok(last_closed_price)
    } else {
        let error = error_handler(result, None).await;
        if error == "E03: Error 502, exchange server is in trouble." {
            Err(error)
        } else if error == "E07: Dns error: No such host is known."
            || error == "E08: Timestamp for this request is outside of the recvWindow"
        {
            get_candle_last_minute_max_value().await
        } else {
            println!("{}", error);
            std::process::exit(1);
        }
    }
}

/// Get the maximum values from a specified number of one-minute closed candles.
///
/// This function retrieves a specified quantity of one-minute closed candles for the specified trading pair (e.g., BTCUSDT)
/// and returns a mapping of timestamps to the maximum (high) values for each candle.
///
/// Parameters:
/// - `quantity`: The number of candles to retrieve.
///
/// Returns:
/// - `Ok(BTreeMap<i64, f64>)`: A mapping of timestamps to maximum values for each candle.
/// - `Err(String)`: An error message if the request fails.
///
#[async_recursion]
pub async fn get_some_1m_candle_max_value(quantity: i64) -> Result<BTreeMap<i64, f64>, String> {
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
        Ok(response) => response,
        Err(_) => re_send_request(client, request, "GET").await,
    };
    if result.status() == StatusCode::OK {
        let data: Vec<KlineData> = result.json().await.unwrap();
        let price_data: Vec<f64> = data
            .iter()
            .take(quantity as usize)
            .map(|f| f.high)
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
            get_some_1m_candle_max_value(quantity).await
        } else {
            println!("{}", error);
            std::process::exit(1);
        }
    }
}

/// Get maximum values for a specified quantity of candles with a custom interval.
///
/// This function retrieves a specified quantity of candles with a custom interval
/// (e.g., "15m" for 15-minute candles) for the specified trading pair (e.g., BTCUSDT).
/// It returns a vector of maximum (high) values for each of the retrieved candles.
///
/// Parameters:
/// - `quantity`: The number of candles to retrieve.
/// - `symbol`: The trading pair symbol (e.g., "BTCUSDT").
/// - `interval`: The custom interval for candles (e.g., "15m" for 15-minute candles).
///
/// Returns:
/// - `Ok(Vec<f64>)`: A vector of maximum values for each retrieved candle.
/// - `Err(String)`: An error message if the request fails.
///
pub async fn get_candle_info_max_value(
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
        one_min_quantity = (quantity + 2) as i64 * candle_length.parse::<i64>().unwrap();
    } else if period == 'h' {
        one_min_quantity = ((quantity + 2) as i64) * 60 * candle_length.parse::<i64>().unwrap();
    // } else if period == 'd' {
    //     one_min_quantity =
    //         ((quantity + 2) as i64) * 60 * 24 * candle_length.parse::<i64>().unwrap();
    } else {
        //if the interval is not valid, the number of candles requested will be "quantity".
        panic!("get_candle_info: Interval not implemented.");
    }

    // Getting exchange candles
    let candle_1m_result = get_some_1m_candle_max_value(one_min_quantity).await;
    //let candle_1m: BTreeMap<i64, f64>;
    if let Ok(candle_1m) = candle_1m_result {
        // Define the desired time frame
        let candle_length = candle_length.parse::<i64>().unwrap();

        // Building requested candles
        let mut candles: Vec<f64> = Vec::new();
        let mut max_value: f64 = 0.0;
        let i = 0;
        let mut is_opened = false;

        for (date, price) in candle_1m {
            // New candle opening
            let data_in_seconds = date / 1000;
            if data_in_seconds % ((one_min_quantity / (quantity + 2) as i64) * 60) == 0 {
                if is_opened {
                    candles.push(max_value);
                    max_value = f64::MIN;
                }
                is_opened = true;
            }

            // Track the maximum value
            if price > max_value {
                max_value = price;
            }
        }

        // Add the last max value to the candles if necessary
        if is_opened {
            candles.push(max_value);
        }
        candles.remove(0);
        candles.pop();

        Ok(candles)
    } else {
        // Handle the error from retrieving the 1-hour candle data
        eprintln!("Failed to retrieve candles: {:?}", candle_1m_result);
        Err("Failed to retrieve candles".to_string())
    }
}

/// Get maximum values for a specified quantity of candles with a custom interval from Binance (just
/// binance intervals because it gets directly from there).
///
/// This function retrieves a specified quantity of candles with a custom interval
/// (e.g., "15m" for 15-minute candles) for the specified trading pair (e.g., BTCUSDT) from Binance.
/// It returns a mapping of timestamps to the maximum (high) values for each of the retrieved candles.
///
/// Parameters:
/// - `quantity`: The number of candles to retrieve.
/// - `interval`: The custom interval for candles (e.g., "15m" for 15-minute candles).
///
/// Returns:
/// - `Ok(BTreeMap<i64, f64>)`: A mapping of timestamps to maximum values for each retrieved candle.
/// - `Err(String)`: An error message if the request fails.
///
#[async_recursion]
pub async fn get_some_candles_from_binance_max_value(
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
    // } else if period == 'd' {
    //     one_min_quantity = (quantity + 2) * 60 * 24 * candle_length.parse::<i64>().unwrap();
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
        let price_data: Vec<f64> = data
            .iter()
            .take(quantity as usize)
            .map(|f| f.high)
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
            get_some_candles_from_binance_max_value(quantity, interval).await
        } else {
            println!("{}", error);
            std::process::exit(1);
        }
    }
}

/// Build candles with the maximum value for a specified quantity and custom interval.
///
/// This function builds candles with the maximum (high) value for a specified quantity
/// and custom interval (e.g., "15m" for 15-hour candles) from one-hour candles.
/// It returns a vector of maximum values for each of the built candles.
///
/// Parameters:
/// - `quantity`: The number of candles to build.
/// - `symbol`: The trading pair symbol (e.g., "BTCUSDT").
/// - `interval`: The custom interval for candles (e.g., "3h" for 3-hour candles).
///
/// Returns:
/// - `Ok(Vec<f64>)`: A vector of maximum values for each built candle.
/// - `Err(String)`: An error message if the request fails.
///
pub async fn build_candle_w_1hr_max_price(
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
            ((quantity + 2) as i64) * 60 * 24 * candle_length.parse::<i64>().unwrap();
    } else {
        panic!("build_candle_w_1hr_max_price: Interval not implemented.");
    }

    // Getting exchange candles
    let candle_1m_result = get_some_1hr_candle_max_value(one_min_quantity).await;

    if let Ok(candle_1m) = candle_1m_result {
        // Define the desired time frame
        let candle_length = candle_length.parse::<i64>().unwrap();

        // Building requested candles
        let mut candles: Vec<f64> = Vec::new();
        let mut max_value: f64 = 0.0;
        let i = 0;
        let mut is_opened = false;

        for (date, price) in candle_1m {
            // New candle opening
            let data_in_seconds = date / 1000;
            if data_in_seconds % ((one_min_quantity / (quantity + 2) as i64) * 60) == 0 {
                if is_opened {
                    candles.push(max_value);
                    max_value = 0.0;
                }
                is_opened = true;
            }

            // Track the maximum value
            if price > max_value {
                max_value = price;
            }
        }

        // Add the last max value to the candles if necessary
        if is_opened {
            candles.push(max_value);
        }

        candles.remove(0);
        candles.pop();

        Ok(candles)
    } else {
        // Handle the error from retrieving the 1-hour candle data
        eprintln!("Failed to retrieve 1-hour candles: {:?}", candle_1m_result);
        Err("Failed to retrieve 1-hour candles".to_string())
    }
}

/// Get the maximum values from a specified number of one-hour closed candles.
///
/// This function retrieves a specified quantity of one-hour closed candles for the specified trading pair (e.g., BTCUSDT)
/// and returns a mapping of timestamps to the maximum (high) values for each candle.
///
/// Parameters:
/// - `quantity`: The number of candles to retrieve.
///
/// Returns:
/// - `Ok(BTreeMap<i64, f64>)`: A mapping of timestamps to maximum values for each candle.
/// - `Err(String)`: An error message if the request fails.
///
#[async_recursion]
pub async fn get_some_1hr_candle_max_value(quantity: i64) -> Result<BTreeMap<i64, f64>, String> {
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
        let price_data: Vec<f64> = data
            .iter()
            .take(quantity as usize)
            .map(|f| f.high)
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
            get_some_1hr_candle_max_value(quantity).await
        } else {
            println!("{}", error);
            std::process::exit(1);
        }
    }
}

/// Get the maximum value from a specified quantity of candles with a custom interval.
///
/// This function retrieves a specified quantity of candles with a custom interval
/// (e.g., "15m" for 15-minute candles) and returns the maximum (high) value among them.
///
/// Parameters:
/// - `quantity`: The number of candles to retrieve.
/// - `interval`: The custom interval for candles (e.g., "15m" for 15-minute candles).
///
/// Returns:
/// - `f64`: The maximum value among the retrieved candles.
///
pub async fn get_biggest_candle(quantity: i64, interval: &str) -> f64 {
    let data = get_some_candles_from_binance_max_value(quantity, interval)
        .await
        .unwrap();

    let mut max_price = 0.0;
    for (date, close_price) in data {
        if max_price < close_price {
            max_price = close_price;
        }
    }
    max_price
}

//Functions tests
#[cfg(test)]
mod tests {
    use super::*;
    use tokio::test;

    /// Test the `get_candle_last_minute_max_value` function.
    ///
    /// This test verifies that the `get_candle_last_minute_max_value` function returns a result with a maximum value greater than 0.0.
    #[test]
    async fn get_candle_last_minute_max_value_test() {
        let res = get_candle_last_minute_max_value().await;
        assert!(res.is_ok());
        let res_unwrapped = res.unwrap();
        assert!(res_unwrapped > 0.0);
    }

    /// Test the `get_some_1m_candle_max_value` function.
    ///
    /// This test verifies that the `get_some_1m_candle_max_value` function returns a valid result with the correct number of candles and ordinate timestamps.
    #[test]
    async fn get_some_1m_candle_max_value_test() {
        let res = get_some_1m_candle_max_value(10).await;
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

    /// Test the `get_candle_info_max_value` function with minutes interval.
    ///
    /// This test verifies that the `get_candle_info_max_value` function returns a valid result with the correct number of candles and valid values for a minutes interval.
    #[test]
    async fn get_candle_info_max_value_minutes_test() {
        let res = get_candle_info_max_value(7, "BTCUSDT", "30m".to_string()).await;
        assert!(res.is_ok());

        let res_unwrapped = res.unwrap();

        //Asserting we received the correct length
        assert!(res_unwrapped.len() == 7);

        for value in res_unwrapped {
            //Asserting that the value makes sense
            assert!(value > 0.0);
        }
    }

    /// Test the `get_candle_info_max_value` function with hours interval.
    ///
    /// This test verifies that the `get_candle_info_max_value` function returns a valid result with the correct number of candles and valid values for an hours interval.
    #[test]
    async fn get_candle_info_max_value_hours_test() {
        let res = get_candle_info_max_value(7, "BTCUSDT", "1h".to_string()).await;
        assert!(res.is_ok());

        let res_unwrapped = res.unwrap();

        //Asserting we received the correct length
        assert!(res_unwrapped.len() == 7);

        for value in res_unwrapped {
            //Asserting that the value makes sense
            assert!(value > 0.0);
        }
    }

    /// Test the `get_some_candles_from_binance_max_value` function with hours interval.
    ///
    /// This test verifies that the `get_some_candles_from_binance_max_value` function returns a valid result with the correct number of candles and ordinate timestamps for an hours interval.
    #[test]
    async fn get_some_candles_from_binance_max_value_hours_test() {
        let res = get_some_candles_from_binance_max_value(7, "1h").await;
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

    /// Test the `get_some_candles_from_binance_max_value` function with minutes interval.
    ///
    /// This test verifies that the `get_some_candles_from_binance_max_value` function returns a valid result with the correct number of candles, valid values, and ordinate timestamps for a minutes interval.
    #[test]
    async fn get_some_candles_from_binance_max_value_minutes_test() {
        let res = get_some_candles_from_binance_max_value(7, "30m").await;
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

    /// Test the `build_candle_w_1hr_max_price` function.
    ///
    /// This test verifies that the `build_candle_w_1hr_max_price` function returns a valid result with the correct number of candles and valid values for a custom interval.
    #[tokio::test]
    async fn test_build_candle_w_1hr_max_price() {
        // Chame a função que você está testando
        let result = build_candle_w_1hr_max_price(16, "BTCUSDT", "3h".to_string()).await;

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

    /// Test the `get_biggest_candle` function.
    ///
    /// This test verifies that the `get_biggest_candle` function returns a maximum value greater than 0.0 for a specified quantity and interval.
    #[test]
    async fn get_biggest_candle_test() {
        let res: f64 = get_biggest_candle(3, "30m").await;
        assert!(res > 0.0);
    }
}
