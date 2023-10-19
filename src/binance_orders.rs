// binance_orders.rs - Binance Order Functions

// This file contains the functions related to order placement, execution, and retrieval of order information
// in a Binance trading bot project.

// The binance_orders.rs file is a crucial component of a trading bot project focused on the Binance exchange.
// It provides the necessary functionality to interact with the exchange's order placement and retrieval system.
// By organizing these functions in a separate file, it promotes code modularity and maintainability,
// making it easier to manage and extend the bot's order-related capabilities.

// Note: Make sure to handle API authentication securely, following best practices to protect sensitive
// information such as API keys and secret keys.

use crate::convert_to_formatted_string;
use crate::error;
use crate::get_candles;
use async_recursion::async_recursion;
use binance_spot_connector_rust::http::request;
use futures_util::future::BoxFuture;
use reqwest::Client;
use serde_json::Value;
use urlencoding::encode;

use serde::Deserialize;
use substring::Substring;

use error::*;
use get_candles::{get_candle_info, get_candle_last_min};

static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));
use hmac::{Hmac, Mac, NewMac};
use json::JsonValue;
use reqwest::{header, Response, StatusCode};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use sha2::Sha256;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{env, string};

pub const QUANTITY_IN_DOLLAR: u64 = 50; //Value that witch strategy will use in the orders (in dollar).

#[derive(Debug, Deserialize, Clone)]
pub struct ResultResponseBinance {
    code: i32,
    msg: String,
}

pub async fn exchange_url() -> String {
    // Verifica se estamos em um ambiente de teste
    let is_test = env::var("RUST_TEST").is_ok();

    if is_test {
        // println!("is");
        // println!("holly: {}", env::var("TEST_API_URL").unwrap());
        env::var("TEST_API_URL").unwrap()
    } else {
        //println!("no");
        env::var("BINANCE_BASE_URL").unwrap()
    }
}

/// In the Binance futures api, the amount that will be invested in each order is in BTC. So, it is necessary to
/// convert the amount in USDT to an BTC quantity. That process is done here.
pub async fn calculate_quantity_in_btc(min_price: bool) -> f64 {
    //Get current price
    let btc_in_dollar_string: String = price_ticker("BTCUSDT".to_string()).await;

    //Converting to float
    let btc_in_dollar_string_without_quotes = btc_in_dollar_string.replace('"', "");
    let btc_in_dollar = btc_in_dollar_string_without_quotes.parse::<f64>().unwrap();

    //Result is the quantity of BTC that we will buy.
    let mut result = QUANTITY_IN_DOLLAR as f64 / btc_in_dollar;
    //Truncating the result.
    let result_with_precision = format!("{:.3}", result);
    result = result_with_precision.parse::<f64>().unwrap();

    if result == 0.000 {
        println!("{}", ERROR_NOT_VALID_QUANTITY);
        std::process::exit(1);
    }

    if min_price {
        0.001
    } else {
        result
    }
}

/// Get binance client. It is necessary to stablish communication if the exchange.
pub async fn get_client() -> reqwest::Client {
    dotenv::dotenv().ok();

    let mut headers = header::HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("application/x-www-form-urlencoded"),
    );

    headers.insert(
        header::HeaderName::from_static("x-mbx-apikey"),
        header::HeaderValue::from_str(&env::var("BINANCE_API_KEY").unwrap()).unwrap(),
    );

    reqwest::Client::builder()
        .default_headers(headers)
        .user_agent(APP_USER_AGENT)
        .build()
        .unwrap()
}

/// Get actual time stamp.
///
/// Parameters:
/// - SystemTime: It is the actual time of the system. ( ex: SystemTime::now() );
///
pub async fn get_timestamp(time: SystemTime) -> u128 {
    let since_epoch = time.duration_since(UNIX_EPOCH).unwrap();
    since_epoch.as_millis()
}

/// Sign a request using Users secret key.
///
/// Parameters:
/// - request: It is the actual time of the system. ( ex: SystemTime::now() );
///
pub async fn get_signature(request: String) -> String {
    let secret_key = env::var("BINANCE_SECRET_KEY").unwrap();
    let mut signed_key = Hmac::<Sha256>::new_from_slice(secret_key.as_bytes()).unwrap();
    signed_key.update(request.as_bytes());

    hex::encode(signed_key.finalize().into_bytes())
}

/// Function that place a new order in the exchange.
///
/// Parameters:
/// - price_order: it is the price - 1 of the order that will be executed.
/// - last_order_id: mutable reference that will store the order id.
/// - is_buy_order: bool that indicates with the order will be buy or sell.
///
#[async_recursion]
pub async fn new_order(
    price_order: f64,
    last_order_id: &mut u64,
    is_buy_order: bool,
    is_reduce_only: bool,
    position_side: Option<String>,
) -> String {
    let new_price_order: Decimal;
    let buy_or_sell: String;
    let quantity = calculate_quantity_in_btc(true).await;

    let mut p_side = "BOTH".to_string();
    let temp_position_side = position_side.clone();
    if temp_position_side.is_some() {
        let unwrapped_position_side = temp_position_side.unwrap();
        if unwrapped_position_side == "LONG" {
            p_side = "LONG".to_string();

            if is_buy_order {
                new_price_order =
                    (Decimal::from_f64_retain(price_order + 1.0).unwrap() * dec!(100)).trunc()
                        / dec!(100);
            } else {
                new_price_order = (Decimal::from_f64_retain(price_order).unwrap() * dec!(100))
                    .trunc()
                    / dec!(100);
            }
        } else {
            p_side = "SHORT".to_string();

            if !is_buy_order {
                new_price_order =
                    (Decimal::from_f64_retain(price_order - 1.0).unwrap() * dec!(100)).trunc()
                        / dec!(100);
            } else {
                new_price_order = (Decimal::from_f64_retain(price_order).unwrap() * dec!(100))
                    .trunc()
                    / dec!(100);
            }
        }
        if is_buy_order {
            buy_or_sell = "BUY".to_string();
        } else {
            buy_or_sell = "SELL".to_string();
        }
    } else if is_buy_order {
        new_price_order =
            (Decimal::from_f64_retain(price_order + 1.0).unwrap() * dec!(100)).trunc() / dec!(100);
        buy_or_sell = "BUY".to_string();
    } else {
        new_price_order =
            (Decimal::from_f64_retain(price_order - 1.0).unwrap() * dec!(100)).trunc() / dec!(100);
        buy_or_sell = "SELL".to_string();
    }

    let client: reqwest::Client = get_client().await;
    let timestamp = get_timestamp(SystemTime::now()).await;

    let mut params = format!(
            "symbol=BTCUSDT&side={}&type=STOP_MARKET&stopPrice={}&timeInForce=GTC&quantity={}&timestamp={}&reduceOnly={}&recvWindow=50000&positionSide={}",
            buy_or_sell,new_price_order, quantity, timestamp, is_reduce_only, p_side
        );
    if p_side != "BOTH" {
        params = format!(
            "symbol=BTCUSDT&side={}&type=STOP_MARKET&stopPrice={}&timeInForce=GTC&quantity={}&timestamp={}&recvWindow=50000&positionSide={}",
            buy_or_sell,new_price_order, quantity, timestamp, p_side
        );
    }

    println!("{}", params);

    let signature = get_signature(params.clone()).await;

    let request = format!(
        "{}/fapi/v1/order?{}&signature={}",
        exchange_url().await,
        params.clone(),
        signature.clone()
    );

    let result = match client.post(request.clone()).send().await {
        Ok(response) => response,
        Err(_) => re_send_request(client, request, "POST").await,
    };

    account_trade_info().await;

    let status: StatusCode = result.status();
    if status == StatusCode::OK {
        let data: serde_json::Value = result.json().await.unwrap();
        println!("Order data: {}", data);
        let temp = data["updateTime"].to_string().parse::<u128>().unwrap();
        let time = convert_to_formatted_string(temp).await;
        println!("{}", time);
        *last_order_id = data["orderId"].to_string().parse().unwrap();
        status.to_string()
    } else {
        let error = error_handler(result, None).await;

        if error == "E01: Order would immediately trigger." {
            new_order_market(last_order_id, is_buy_order, p_side).await
        } else if error == "E03: Error 502, exchange server is in trouble." {
            error
        } else if error == "E07: Dns error: No such host is known."
            || error == "E08: Timestamp for this request is outside of the recvWindow"
        {
            new_order(
                price_order,
                last_order_id,
                is_buy_order,
                is_reduce_only,
                position_side,
            )
            .await
        } else {
            std::process::exit(1);
        }
    }
}

/// Function that place a new order limit in the exchange.
///
/// Parameters:
/// - price_order: it is the price - 1 of the order that will be executed.
/// - last_order_id: mutable reference that will store the order id.
/// - is_buy_order: bool that indicates with the order will be buy or sell.
/// - position_side: Option<String> that represent the side (long, short or both).
///
#[async_recursion]
pub async fn new_order_limit(
    price_order: f64,
    last_order_id: &mut u64,
    is_buy_order: bool,
    position_side: Option<String>,
) -> String {
    let new_price_order: Decimal;
    let buy_or_sell: String;
    //let price_order: f64 = 30000.0;

    //Getting quantity in BTC.
    let quantity = calculate_quantity_in_btc(true).await;

    let mut p_side = "BOTH".to_string();
    let temp_position_side = position_side.clone();

    if temp_position_side.is_some() {
        let unwrapped_position_side = temp_position_side.unwrap();
        if unwrapped_position_side == "LONG" {
            p_side = "LONG".to_string();
            new_price_order = (Decimal::from_f64_retain(price_order + 1.0).unwrap() * dec!(10))
                .trunc()
                / dec!(10);
        } else {
            p_side = "SHORT".to_string();
            new_price_order = (Decimal::from_f64_retain(price_order - 1.0).unwrap() * dec!(10))
                .trunc()
                / dec!(10);
        }
        if is_buy_order {
            buy_or_sell = "BUY".to_string();
        } else {
            buy_or_sell = "SELL".to_string();
        }
    } else if is_buy_order {
        new_price_order =
            (Decimal::from_f64_retain(price_order + 1.0).unwrap() * dec!(10)).trunc() / dec!(10);
        buy_or_sell = "BUY".to_string();
    } else {
        new_price_order =
            (Decimal::from_f64_retain(price_order - 1.0).unwrap() * dec!(10)).trunc() / dec!(10);
        buy_or_sell = "SELL".to_string();
        //quantity *= 100_f64;
    }

    let client: reqwest::Client = get_client().await;
    let timestamp = get_timestamp(SystemTime::now()).await;

    let params = format!(
            "symbol=BTCUSDT&side={}&type={}&price={}&timeInForce=GTC&quantity={}&timestamp={}&recvWindow=50000&positionSide={}",
            buy_or_sell,"LIMIT",new_price_order,quantity, timestamp, p_side
        );
    println!("params: {}", params);
    let signature = get_signature(params.clone()).await;

    let request = format!(
        "{}/fapi/v1/order?{}&signature={}",
        exchange_url().await,
        params.clone(),
        signature.clone()
    );

    let result = match client.post(request.clone()).send().await {
        Ok(response) => response,
        Err(_) => re_send_request(client, request, "POST").await,
    };
    account_trade_info().await;

    let status = result.status();
    if status == StatusCode::OK {
        let data: serde_json::Value = result.json().await.unwrap();
        println!("Order data: {}", data);
        let temp = data["updateTime"].to_string().parse::<u128>().unwrap();
        let time = convert_to_formatted_string(temp).await;
        println!("{}", time);
        *last_order_id = data["orderId"].to_string().parse().unwrap();
        status.to_string()
    } else {
        let error = error_handler(result, None).await;
        if error == "E03: Error 502, exchange server is in trouble." {
            error
        } else if error == "E07: Dns error: No such host is known."
            || error == "E08: Timestamp for this request is outside of the recvWindow"
        {
            new_order_limit(price_order, last_order_id, is_buy_order, position_side).await
        } else {
            println!("{}", error);
            std::process::exit(1);
        }
    }
}

/// Function that place a new order market in the exchange.
///
/// Parameters:
/// - last_order_id: mutable reference that will store the order id.
/// - is_buy_order: bool that indicates with the order will be buy or sell.
/// - position_side: Option<String> that represent the side (long, short or both).
///
#[async_recursion]
pub async fn new_order_market(
    last_order_id: &mut u64,
    is_buy_order: bool,
    position_side: String,
) -> String {
    let mut buy_or_sell: String = "SELL".to_string();
    //Getting quantity in BTC.
    let quantity = calculate_quantity_in_btc(true).await;

    if is_buy_order {
        buy_or_sell = "BUY".to_string();
    }
    let client: reqwest::Client = get_client().await;
    let timestamp = get_timestamp(SystemTime::now()).await;

    let params = format!(
        "symbol=BTCUSDT&side={}&type={}&quantity={}&timestamp={}&recvWindow=50000&positionSide={}",
        buy_or_sell, "MARKET", quantity, timestamp, position_side
    );

    let signature = get_signature(params.clone()).await;

    let request = format!(
        "{}/fapi/v1/order?{}&signature={}",
        exchange_url().await,
        params.clone(),
        signature.clone()
    );
    let result = match client.post(request.clone()).send().await {
        Ok(response) => response,
        Err(_) => re_send_request(client, request, "POST").await,
    };
    let status = result.status();

    account_trade_info().await;

    if status == StatusCode::OK {
        let data: serde_json::Value = result.json().await.unwrap();
        println!("Order data: {}", data);
        let temp = data["updateTime"].to_string().parse::<u128>().unwrap();
        let time = convert_to_formatted_string(temp).await;
        println!("{}", time);
        *last_order_id = data["orderId"].to_string().parse().unwrap();
        status.to_string()
    } else {
        let error = error_handler(result, None).await;
        if error == "E03: Error 502, exchange server is in trouble." {
            error
        } else if error == "E07: Dns error: No such host is known."
            || error == "E08: Timestamp for this request is outside of the recvWindow"
        {
            new_order_market(last_order_id, is_buy_order, position_side).await
        } else {
            println!("{}", error);
            std::process::exit(1);
        }
    }
}

/// Cancel old order and place another one.
///
/// Parameters:
/// - price_order: it is the price of the order that will be executed.
/// - order_id: mutable reference that contain the last order id executed.
/// - is_buy_order: bool that indicates with the order will be buy or sell.
///

#[async_recursion]
pub async fn cancel_an_existing_order_and_send_a_new_order(
    price_order: f64,
    order_id: &mut u64,
    is_buy_order: bool,
    is_reduce_only: bool,
    position_side: Option<String>,
) -> String {
    let client: reqwest::Client = get_client().await;
    // Cancel the order
    let timestamp = get_timestamp(SystemTime::now()).await;
    let params = format!(
        "symbol=BTCUSDT&orderId={}&timestamp={}&recvWindow=50000",
        order_id, timestamp
    );
    let signature = get_signature(params.clone()).await;
    let request = format!(
        "{}/fapi/v1/order?{}&signature={}",
        exchange_url().await,
        params.clone(),
        signature.clone()
    );
    // Sending HTTP delete will cancel the order
    let result = match client.delete(request.clone()).send().await {
        Ok(response) => response,
        Err(_) => re_send_request(client, request, "DELETE").await,
    };
    let status: StatusCode = result.status();

    if status == StatusCode::OK {
        let _data: serde_json::Value = result.json().await.unwrap();

        new_order(
            price_order,
            order_id,
            is_buy_order,
            is_reduce_only,
            position_side,
        )
        .await
    } else {
        let error = error_handler(result, None).await;
        if error == "E03: Error 502, exchange server is in trouble." {
            error
        } else if error == "E07: Dns error: No such host is known."
            || error == "E08: Timestamp for this request is outside of the recvWindow"
        {
            cancel_an_existing_order_and_send_a_new_order(
                price_order,
                order_id,
                is_buy_order,
                is_reduce_only,
                position_side,
            )
            .await
        } else {
            std::process::exit(1);
        }
    }
}

/// Function that cancel all open orders in the user's binance account.
#[async_recursion]
pub async fn cancel_all_open_orders() -> String {
    let client: reqwest::Client = get_client().await;
    let timestamp = get_timestamp(SystemTime::now()).await;
    let params = format!("symbol=BTCUSDT&timestamp={}", timestamp);
    let signature = get_signature(params.clone()).await;

    let request = format!(
        "{}/fapi/v1/allOpenOrders?{}&signature={}",
        exchange_url().await,
        params.clone(),
        signature.clone()
    );

    let result = match client.delete(request.clone()).send().await {
        Ok(response) => response,
        Err(_) => re_send_request(client, request, "DELETE").await,
    };

    let status = result.status();
    if status == StatusCode::OK {
        let _data: serde_json::Value = result.json().await.unwrap();
        //println!("Order data: {}", data);
        "No more open orders.".to_string()
    } else {
        let error = error_handler(result, None).await;
        if error == "E03: Error 502, exchange server is in trouble." {
            error
        } else if error == "E07: Dns error: No such host is known."
            || error == "E08: Timestamp for this request is outside of the recvWindow"
        {
            cancel_all_open_orders().await
        } else {
            println!("{}", error);
            std::process::exit(1);
        }
    }
}

/// Function that cancel all open orders in the user's binance account
/// and does not look for errors.
pub async fn cancel_all_open_orders_without_error_check() {
    let client: reqwest::Client = get_client().await;
    let timestamp = get_timestamp(SystemTime::now()).await;
    let params = format!("symbol=BTCUSDT&timestamp={}", timestamp);
    let signature = get_signature(params.clone()).await;

    let request = format!(
        "{}/fapi/v1/allOpenOrders?{}&signature={}",
        exchange_url().await,
        params.clone(),
        signature.clone()
    );

    let result = match client.delete(request.clone()).send().await {
        Ok(response) => response,
        Err(_) => re_send_request(client, request, "DELETE").await,
    };
}

/// Retrieves the status of an order with the given order ID.
///
/// This function sends a request to the exchange server to fetch the status of an order
/// identified by the provided `order_id`. It returns a string indicating the order's status.
///
/// # Arguments
/// * `order_id`: A unique identifier for the order.
///
/// # Returns
/// A `String` containing the order's status.
///
#[async_recursion]
pub async fn order_status(order_id: u64) -> String {
    if order_id == 0 {
        return "Invalid Order ID.".to_string();
    }

    let client: reqwest::Client = get_client().await;
    let timestamp = get_timestamp(SystemTime::now()).await;

    let params = format!(
        "orderId={}&symbol=BTCUSDT&timestamp={}&recvWindow=50000",
        order_id, timestamp
    );

    let signature = get_signature(params.clone()).await;

    let request = format!(
        "{}/fapi/v1/order?{}&signature={}",
        exchange_url().await,
        params.clone(),
        signature.clone()
    );
    //println!("req: {}", request);

    let result = match client.get(request.clone()).send().await {
        Ok(response) => response,
        Err(_) => re_send_request(client, request, "GET").await,
    };

    if result.status() == StatusCode::OK {
        let data: serde_json::Value = result.json().await.unwrap();
        //println!("data :{}", data);

        data["status"].to_string().replace('\"', "")
    } else {
        let error = error_handler(result, None).await;
        if error == "E03: Error 502, exchange server is in trouble." {
            error
        } else if error == "E07: Dns error: No such host is known."
            || error == "E08: Timestamp for this request is outside of the recvWindow"
        {
            order_status(order_id).await
        } else {
            println!("{}", error);
            std::process::exit(1);
        }
    }
}

#[async_recursion]
pub async fn get_stop_price(order_id: u64) -> String {
    let client: reqwest::Client = get_client().await;
    let timestamp = get_timestamp(SystemTime::now()).await;
    let params = format!(
        "orderId={}&symbol=BTCUSDT&timestamp={}&recvWindow=50000",
        order_id, timestamp
    );
    let signature = get_signature(params.clone()).await;

    let request = format!(
        "{}/fapi/v1/order?{}&signature={}",
        exchange_url().await,
        params.clone(),
        signature.clone()
    );
    println!("req: {}", request);

    let result = match client.get(request.clone()).send().await {
        Ok(response) => response,
        Err(_) => re_send_request(client, request, "GET").await,
    };
    if result.status() == StatusCode::OK {
        let data: serde_json::Value = result.json().await.unwrap();
        let data_string = data["stopPrice"].to_string();
        println!("data: {}", data_string);
        let str_no_quotes = (data_string).substring(1, data_string.len() - 1);
        let stop_price: f64 = str_no_quotes.parse::<f64>().unwrap();

        stop_price.to_string()
    } else {
        let error = error_handler(result, None).await;
        if error == "E03: Error 502, exchange server is in trouble." {
            error
        } else if error == "E07: Dns error: No such host is known."
            || error == "E08: Timestamp for this request is outside of the recvWindow"
        {
            get_stop_price(order_id).await
        } else {
            println!("{}", error);
            std::process::exit(1);
        }
    }
}

/// Retrieves the stop price of an order with the given order ID.
///
/// This function sends a request to the exchange server to fetch the stop price of an order
/// identified by the provided `order_id`. It returns a string representation of the stop price.
///
/// # Arguments
/// * `order_id`: A unique identifier for the order.
///
/// # Returns
/// A `String` containing the stop price of the order.
///
#[async_recursion]
pub async fn cancel_open_order(order_id: u64) -> String {
    let client: reqwest::Client = get_client().await;
    // Cancel the order
    let timestamp = get_timestamp(SystemTime::now()).await;
    let params = format!(
        "symbol=BTCUSDT&orderId={}&timestamp={}&recvWindow=50000",
        order_id, timestamp
    );
    let signature = get_signature(params.clone()).await;
    let request = format!(
        "{}/fapi/v1/order?{}&signature={}",
        exchange_url().await,
        params.clone(),
        signature.clone()
    );
    // Sending HTTP delete will cancel the order
    let result = match client.delete(request.clone()).send().await {
        Ok(response) => response,
        Err(_) => re_send_request(client, request, "DELETE").await,
    };
    let status = result.status();
    account_trade_info().await;

    if status == StatusCode::OK {
        let _data: serde_json::Value = result.json().await.unwrap();
        //println!("Cancel order data: {}", data);
        let temp = _data["updateTime"].to_string().parse::<u128>().unwrap();
        let time = convert_to_formatted_string(temp).await;
        println!("{}", time);
        status.to_string()
    } else {
        let error = error_handler(result, None).await;
        if error == "E03: Error 502, exchange server is in trouble." {
            cancel_open_order(order_id).await;
            error
        } else if error == "E07: Dns error: No such host is known."
            || error == "E08: Timestamp for this request is outside of the recvWindow"
        {
            cancel_open_order(order_id).await
        } else {
            println!("{}", error);
            std::process::exit(1);
        }
    }
}

/// Tests the connection to the Binance exchange server.
///
/// This function sends a ping request to the Binance exchange server to test the connection.
///
/// # Returns
/// A `String` containing the HTTP status code as a result of the ping request.
///
#[async_recursion]
pub async fn test_binance_connection() -> String {
    let client: reqwest::Client = get_client().await;

    let request = format!("{}/fapi/v1/ping", exchange_url().await);
    // Sending HTTP delete will cancel the order
    let result = match client.get(request.clone()).send().await {
        Ok(response) => response,
        Err(_) => re_send_request(client, request, "GET").await,
    };
    let status = result.status();
    if status == StatusCode::OK {
        status.to_string()
    } else {
        let error = error_handler(result, None).await;
        if error == "E03: Error 502, exchange server is in trouble." {
            error
        } else if error == "E07: Dns error: No such host is known."
            || error == "E08: Timestamp for this request is outside of the recvWindow"
        {
            test_binance_connection().await
        } else {
            println!("{}", error);
            std::process::exit(1);
        }
    }
}

/// Retrieves the open orders for a specific symbol (BTCUSDT) on the Binance exchange.
///
/// This function sends a request to the Binance exchange server to fetch the open orders
/// for the specified symbol (BTCUSDT). It returns a string representation of the JSON response
/// containing open order information.
///
/// # Returns
/// A `String` containing the JSON response with open order information.
///
#[async_recursion]
pub async fn binance_open_orders() -> Result<Value, String> {
    let client: reqwest::Client = get_client().await;
    // Cancel the order
    let timestamp = get_timestamp(SystemTime::now()).await;
    let params = format!("symbol=BTCUSDT&timestamp={}&recvWindow=50000", timestamp);
    let signature = get_signature(params.clone()).await;
    let request = format!(
        "{}/fapi/v1/openOrders?{}&signature={}",
        exchange_url().await,
        params.clone(),
        signature.clone()
    );
    // Sending HTTP delete will cancel the order
    let result = match client.get(request.clone()).send().await {
        Ok(response) => response,
        Err(_) => re_send_request(client, request, "GET").await,
    };
    let status = result.status();
    if status == StatusCode::OK {
        let data: serde_json::Value = result.json().await.unwrap();
        //println!("Data: {}", data);
        //data.to_string()
        Ok(data)
    } else {
        let error = error_handler(result, None).await;
        if error == "E03: Error 502, exchange server is in trouble." {
            Err(error)
        } else if error == "E07: Dns error: No such host is known."
            || error == "E08: Timestamp for this request is outside of the recvWindow"
        {
            binance_open_orders().await
        } else {
            println!("{}", error);
            std::process::exit(1);
        }
    }
}

/// Retrieves exchange information for a specific symbol on the Binance exchange.
///
/// This function sends a request to the Binance exchange server to fetch information about
/// the the exchange.
///
/// # Returns
/// A `String` containing the JSON response with exchange information.
///
#[async_recursion]
pub async fn exchange_info() -> String {
    let client: reqwest::Client = get_client().await;
    let timestamp = get_timestamp(SystemTime::now()).await;

    let params = format!("symbol=BTCUSDT&timestamp={}&recvWindow=50000", timestamp);

    let signature = get_signature(params.clone()).await;

    let request = format!(
        "{}/fapi/v1/exchangeInfo?{}&signature={}",
        exchange_url().await,
        params.clone(),
        signature.clone()
    );

    // let request = "{}/fapi/v1/exchangeInfo".to_string();

    let result = match client.get(request.clone()).send().await {
        Ok(response) => response,
        Err(_) => re_send_request(client, request, "GET").await,
    };
    if result.status() == StatusCode::OK {
        let data: serde_json::Value = result.json().await.unwrap();
        //println!("{}", data);
        data.to_string()
    } else {
        let error = error_handler(result, None).await;
        if error == "E03: Error 502, exchange server is in trouble." {
            error
        } else if error == "E07: Dns error: No such host is known."
            || error == "E08: Timestamp for this request is outside of the recvWindow"
        {
            exchange_info().await
        } else {
            println!("{}", error);
            std::process::exit(1);
        }
    }
}

/// Retrieves the price ticker for a specific trading symbol on the Binance exchange.
///
/// This function sends a request to the Binance exchange server to fetch the price ticker
/// for the specified trading symbol. It returns a string representation of the current price
/// for that symbol.
///
/// # Arguments
/// * `symbol`: A string representing the trading symbol (e.g., "BTCUSDT").
///
/// # Returns
/// A `String` containing the current price for the specified symbol.
///
#[async_recursion]
pub async fn price_ticker(symbol: String) -> String {
    let client: reqwest::Client = get_client().await;

    let params = format!("symbol={}", symbol);

    let signature = get_signature(params.clone()).await;

    let request = format!(
        "{}/fapi/v1/ticker/price?{}&signature={}",
        exchange_url().await,
        params.clone(),
        signature.clone()
    );

    let result = match client.get(request.clone()).send().await {
        Ok(response) => response,
        Err(_) => re_send_request(client, request, "GET").await,
    };
    if result.status() == StatusCode::OK {
        let data: serde_json::Value = result.json().await.unwrap();
        // println!("{}", data);
        // println!("{}", data["price"]);

        data["price"].to_string()
    } else {
        let error = error_handler(result, None).await;
        if error == "E03: Error 502, exchange server is in trouble." {
            error
        } else if error == "E07: Dns error: No such host is known."
            || error == "E08: Timestamp for this request is outside of the recvWindow"
        {
            price_ticker(symbol).await
        } else {
            println!("{}", error);
            std::process::exit(1);
        }
    }
}

//
/// Retrieves position information for a specific trading symbol on the Binance exchange.
///
/// This function sends a request to the Binance exchange server to fetch position information
/// for the specified trading symbol (BTCUSDT). It returns a string representation of the JSON response
/// containing position details.
///
/// # Returns
/// A `String` containing the JSON response with position information.
#[async_recursion]
pub async fn position_info() -> Result<serde_json::Value, String> {
    let client: reqwest::Client = get_client().await;
    let timestamp = get_timestamp(SystemTime::now()).await;

    let params = format!("symbol=BTCUSDT&timestamp={}&recvWindow=50000", timestamp);

    let signature = get_signature(params.clone()).await;

    let request = format!(
        "{}/fapi/v2/positionRisk?{}&signature={}",
        exchange_url().await,
        params.clone(),
        signature.clone()
    );
    let result = match client.get(request.clone()).send().await {
        Ok(response) => response,
        Err(_) => re_send_request(client, request, "GET").await,
    };
    let status = result.status();

    if status == StatusCode::OK {
        let data: serde_json::Value = result.json().await.unwrap();
        //println!("Response: {}", data);
        Ok(data)
        //status.to_string()
    } else {
        let error = error_handler(result, None).await;
        if error == "E03: Error 502, exchange server is in trouble." {
            Err(error)
        } else if error == "E07: Dns error: No such host is known."
            || error == "E08: Timestamp for this request is outside of the recvWindow"
        {
            position_info().await
        } else {
            println!("{}", error);
            std::process::exit(1);
        }
    }
}

/// Closes a position on the Binance exchange.
///
/// This function sends a request to the Binance exchange server to close a position. The order type is determined
/// by the `is_buy_order` parameter: if `true`, it's a market buy order; if `false`, it's a market sell order.
///
/// You can specify the `position_side` to indicate whether you want to close a "LONG" or "SHORT" position.
///
/// # Arguments
/// * `is_buy_order`: A boolean indicating whether it's a buy (true) or sell (false) order.
/// * `position_side`: An optional string indicating the position side to close ("LONG," "SHORT," or "BOTH").
///
/// # Returns
/// A `String` containing the status of the order execution.
///
#[async_recursion]
pub async fn close_position(is_buy_order: bool, position_side: Option<String>) -> String {
    let mut buy_or_sell: String = "SELL".to_string();

    //Getting quantity in BTC.
    let quantity = calculate_quantity_in_btc(true).await * 100_f64;

    if is_buy_order {
        buy_or_sell = "BUY".to_string();
    }

    let mut p_side = "BOTH".to_string();

    let clone_position_side = position_side.clone();
    if clone_position_side.is_some() {
        let unwrapped_position_side = clone_position_side.unwrap();
        if unwrapped_position_side == "LONG" {
            p_side = "LONG".to_string();
        } else if unwrapped_position_side == "SHORT" {
            p_side = "SHORT".to_string();
        }
    }
    let client: reqwest::Client = get_client().await;
    let timestamp = get_timestamp(SystemTime::now()).await;

    let mut params = format!(
            "symbol=BTCUSDT&side={}&type=MARKET&quantity={}&timestamp={}&recvWindow=50000&positionSide={}",
            buy_or_sell, quantity, timestamp, p_side
        );
    if p_side == "BOTH" {
        params = format!(
            "symbol=BTCUSDT&side={}&type=STOP_MARKET&timeInForce=GTC&quantity={}&timestamp={}&recvWindow=50000&positionSide={}",
            buy_or_sell, quantity, timestamp, p_side
        );
    }

    let signature = get_signature(params.clone()).await;

    let request = format!(
        "{}/fapi/v1/order?{}&signature={}",
        exchange_url().await,
        params.clone(),
        signature.clone()
    );
    let result = match client.post(request.clone()).send().await {
        Ok(response) => response,
        Err(_) => re_send_request(client, request, "POST").await,
    };
    let status = result.status();

    if status == StatusCode::OK {
        let data: serde_json::Value = result.json().await.unwrap();
        println!("Order data: {}", data);
        let temp = data["updateTime"].to_string().parse::<u128>().unwrap();
        let time = convert_to_formatted_string(temp).await;
        println!("{}", time);
        status.to_string()
    } else {
        let error = error_handler(result, None).await;
        if error == "E03: Error 502, exchange server is in trouble." {
            error
        } else if error == "E05: ReduceOnly Order is rejected." {
            "No position to close. Everything ok.".to_string()
        } else if error == "E07: Dns error: No such host is known."
            || error == "E08: Timestamp for this request is outside of the recvWindow"
        {
            close_position(is_buy_order, position_side).await
        } else {
            println!("{}", error);
            std::process::exit(1);
        }
    }
}

/// Activates the dual side position mode on the Binance exchange.
///
/// This function sends a request to the Binance exchange server to activate dual side position mode.
///
/// # Returns
/// A `String` containing the status of the activation.
///
#[async_recursion]
pub async fn activate_hedge_mode() -> String {
    let client: reqwest::Client = get_client().await;
    let timestamp = get_timestamp(SystemTime::now()).await;

    let params = format!("dualSidePosition=true&timestamp={}", timestamp);

    let signature = get_signature(params.clone()).await;

    let request = format!(
        "{}/fapi/v1/positionSide/dual?{}&signature={}",
        exchange_url().await,
        params.clone(),
        signature.clone()
    );
    let result = match client.post(request.clone()).send().await {
        Ok(response) => response,
        Err(_) => re_send_request(client, request, "POST").await,
    };
    let status = result.status();

    if status == StatusCode::OK {
        let data: serde_json::Value = result.json().await.unwrap();
        println!("Order data: {}", data);
        status.to_string()
    } else {
        let error = error_handler(result, None).await;
        if (error == "E03: Error 502, exchange server is in trouble.")
            || error.contains("No need to change")
        {
            error
        } else if error == "E07: Dns error: No such host is known."
            || error == "E08: Timestamp for this request is outside of the recvWindow"
        {
            activate_hedge_mode().await
        } else {
            println!("{}", error);
            std::process::exit(1);
        }
    }
}

/// Deactivates the dual side position mode on the Binance exchange.
///
/// This function sends a request to the Binance exchange server to deactivate dual side position mode.
///
/// # Returns
/// A `String` containing the status of the deactivation.
///
#[async_recursion]
pub async fn deactivate_hedge_mode() -> String {
    let client: reqwest::Client = get_client().await;
    let timestamp = get_timestamp(SystemTime::now()).await;

    let params = format!("dualSidePosition=false&timestamp={}", timestamp);

    let signature = get_signature(params.clone()).await;

    let request = format!(
        "{}/fapi/v1/positionSide/dual?{}&signature={}",
        exchange_url().await,
        params.clone(),
        signature.clone()
    );
    let result = match client.post(request.clone()).send().await {
        Ok(response) => response,
        Err(_) => re_send_request(client, request, "POST").await,
    };
    let status = result.status();

    if status == StatusCode::OK {
        let data: serde_json::Value = result.json().await.unwrap();
        println!("Order data: {}", data);
        status.to_string()
    } else {
        let error = error_handler(result, None).await;
        if (error == "E03: Error 502, exchange server is in trouble.")
            || error.contains("No need to change")
        {
            error
        } else if error == "E07: Dns error: No such host is known."
            || error == "E08: Timestamp for this request is outside of the recvWindow"
        {
            deactivate_hedge_mode().await
        } else {
            println!("{}", error);
            std::process::exit(1);
        }
    }
}

/// Retrieves detailed information about an order with the given order ID.
///
/// This function sends a request to the Binance exchange server to fetch detailed information
/// about an order identified by the provided `order_id`. It returns a string representation of
/// the JSON response containing order details.
///
/// # Arguments
/// * `order_id`: A unique identifier for the order.
///
/// # Returns
/// A `String` containing the JSON response with order details.
///
#[async_recursion]
pub async fn get_order(order_id: u64) -> String {
    if order_id == 0 {
        return "Invalid Order ID.".to_string();
    }

    let client: reqwest::Client = get_client().await;
    let timestamp = get_timestamp(SystemTime::now()).await;

    let params = format!(
        "orderId={}&symbol=BTCUSDT&timestamp={}&recvWindow=50000",
        order_id, timestamp
    );

    let signature = get_signature(params.clone()).await;

    let request = format!(
        "{}/fapi/v1/order?{}&signature={}",
        exchange_url().await,
        params.clone(),
        signature.clone()
    );

    let result = match client.get(request.clone()).send().await {
        Ok(response) => response,
        Err(_) => re_send_request(client, request, "GET").await,
    };
    if result.status() == StatusCode::OK {
        let data: serde_json::Value = result.json().await.unwrap();
        //println!("data :{}", data);
        data.to_string()
        //data["status"].to_string().replace('\"', "")
    } else {
        let error = error_handler(result, None).await;
        if error == "E03: Error 502, exchange server is in trouble." {
            error
        } else if error == "E07: Dns error: No such host is known."
            || error == "E08: Timestamp for this request is outside of the recvWindow"
        {
            get_order(order_id).await
        } else {
            println!("{}", error);
            std::process::exit(1);
        }
    }
}

/// Checks if a stop order can be placed for a "LONG" position.
///
/// This function determines whether a stop order can be placed for a "LONG" position based on the
/// given `price_order` and the current market price. It is used to not put a trailing that will trigger
/// automatically.
///
/// # Arguments
/// * `price_order`: The price at which the stop order is intended to trigger.
///
/// # Returns
/// A boolean value indicating whether the stop order can be placed for a "LONG" position.
///
pub async fn can_place_stop_order_long(price_order: f64) -> bool {
    //Get current market price
    let res: String = price_ticker("BTCUSDT".to_string()).await.replace('\"', "");
    let market_price = res.parse::<f64>().unwrap();

    //Update trailing if it will not trigger
    price_order < market_price
}

/// Checks if a stop order can be placed for a "SHORT" position.
///
/// This function determines whether a stop order can be placed for a "SHORT" position based on the
/// given `price_order` and the current market price. It is used to not put a trailing that will trigger
/// automatically.
///
/// # Arguments
/// * `price_order`: The price at which the stop order is intended to trigger.
///
/// # Returns
/// A boolean value indicating whether the stop order can be placed for a "SHORT" position.
///
pub async fn can_place_stop_order_short(price_order: f64) -> bool {
    //Get current market price
    let res: String = price_ticker("BTCUSDT".to_string()).await.replace('\"', "");
    let market_price = res.parse::<f64>().unwrap();

    //Update trailing if it will not trigger
    price_order > market_price
}

// let data: serde_json::Value = result.json().await.unwrap();
// let data_string = data["stopPrice"].to_string();

///
///
///
pub async fn account_trade_info() {
    let position_info: serde_json::Value = position_info().await.unwrap();
    //let json: serde_json::Value = position_info.into();
    //println!("ble {} ", position_info);
    let temp1 = position_info.get(1).unwrap();
    let temp0 = position_info.get(0).unwrap();

    let pside = temp0["positionSide"].clone();

    if pside == "SHORT" {
        println!(
            "- Short position has: {} amount in operation.",
            temp0["positionAmt"]
        );
        println!(
            "- Long position has: {} amount in operation.",
            temp1["positionAmt"]
        );
    } else if pside == "LONG" {
        println!(
            "- Short position has: {} amount in operation.",
            temp1["positionAmt"]
        );
        println!(
            "- Long position has: {} amount in operation.",
            temp0["positionAmt"]
        );
    } else if pside == "BOTH" {
        println!(
            "- Both position has: {} amount in operation.",
            temp0["positionAmt"]
        );
    } else {
        println!("- Problem in the code with account_trade_info.");
        std::process::exit(1);
    }

    println!(
        "- The number of open orders now is: {}",
        binance_open_orders()
            .await
            .unwrap()
            .as_array()
            .unwrap()
            .len()
    )
}

#[async_recursion]
pub async fn re_send_request(client: Client, request: String, method: &str) -> Response {
    println!("Re-sending the request!");

    if method == "GET" {
        match client.get(request.clone()).send().await {
            Ok(response) => response,
            Err(_) => re_send_request(client, request, method).await,
        }
    } else if method == "POST" {
        match client.get(request.clone()).send().await {
            Ok(response) => response,
            Err(_) => re_send_request(client, request, method).await,
        }
    } else if method == "DELETE" {
        match client.delete(request.clone()).send().await {
            Ok(response) => response,
            Err(_) => re_send_request(client, request, method).await,
        }
    } else {
        panic!("Invalid method in the re-send request.")
    }
}

// Test Functions
#[cfg(test)]
mod tests {
    use super::*;
    use http;
    use reqwest::Response;
    use serde::__private::de::IdentifierDeserializer;
    use std::thread::sleep;
    use std::time::Duration;
    use tokio::test;

    /// Reset the environment for testing.
    ///
    /// This function sets up a clean testing environment for other test cases.
    ///
    async fn reset_for_test() {
        cancel_all_open_orders().await;

        activate_hedge_mode().await;
        close_position(false, Some("LONG".to_string())).await;
        close_position(true, Some("SHORT".to_string())).await;
        //activate_hedge_mode().await;
    }

    /// Test closing a short position.
    ///
    /// This test function checks the functionality of closing a short position by calling the `close_position`
    /// function with the "SHORT" position side. It first resets the testing environment using `reset_for_test`.
    ///
    #[test]
    async fn close_short_position_test() {
        reset_for_test().await;
        close_position(true, Some("SHORT".to_string())).await;
    }

    /// Test closing a long position.
    ///
    /// This test function checks the functionality of closing a long position by calling the `close_position`
    /// function with the "LONG" position side. It first resets the testing environment using `reset_for_test`.
    ///
    #[test]
    async fn close_long_position_test() {
        reset_for_test().await;
        close_position(false, Some("LONG".to_string())).await;
    }

    /// Test calculating quantity in BTC.
    ///
    /// This test function checks the calculate_quantity_in_btc_test function
    ///
    #[test]
    async fn calculate_quantity_in_btc_test() {
        let res = calculate_quantity_in_btc(true).await;
        assert_eq!(res, 0.001);

        let res = calculate_quantity_in_btc(false).await;
        assert_eq!(res, 0.002);
    }

    /// Test getting an HTTP client.
    ///
    /// This test function verifies the functionality of getting an HTTP client using the `get_client` function.
    ///
    #[test]
    async fn get_client_test() {
        get_client().await;
    }

    /// Test getting a timestamp.
    ///
    /// This test function checks the accuracy of getting a timestamp based on the provided `SystemTime`.
    ///
    #[test]
    async fn get_timestamp_test() {
        let time_now = SystemTime::now();
        let res = get_timestamp(time_now).await;
        assert!(res > 1672531200); // > 2023
    }

    /// Test placing a new order for a long position.
    ///
    /// This test function verifies the functionality of placing a new order for a long position using the `new_order`
    /// function. It checks whether the order is executed successfully with a specific truncated price.
    ///
    #[test]
    async fn new_order_long_test() {
        reset_for_test().await;

        let truncated_price: f64 = 20000.0;
        let res = new_order(
            truncated_price,
            &mut 0,
            true,
            false,
            Some("LONG".to_string()),
        )
        .await;
        assert_eq!(res, "200 OK".to_string());
        close_position(false, Some("LONG".to_string())).await;
    }

    /// Test placing a new order for a short position.
    ///
    /// This test function verifies the functionality of placing a new order for a short position using the `new_order`
    /// function. It checks whether the order is executed successfully with a specific truncated price.
    ///
    #[test]
    async fn new_order_short_test() {
        reset_for_test().await;

        let truncated_price: f64 = 200000.0;
        let res = new_order(
            truncated_price,
            &mut 0,
            false,
            false,
            Some("SHORT".to_string()),
        )
        .await;
        assert_eq!(res, "200 OK".to_string());
        close_position(true, Some("SHORT".to_string())).await;
    }

    /// Test placing a stop order for a long position.
    ///
    /// This test function checks the functionality of placing a stop order for a long position by calling the
    /// `can_place_stop_order_long` function with different price scenarios. It verifies that the function returns
    /// the expected results.
    ///
    #[test]
    async fn can_place_stop_order_long_test() {
        reset_for_test().await;

        //Try to place a long order in a higher price (should work);
        let res = can_place_stop_order_long(1.0).await;
        assert!(res, "Can't place the stop order long.");

        //Try to place a long order in a higher price (should not work);
        let res = can_place_stop_order_long(f64::MAX).await;
        assert!(!res, "Can't place the stop order long.");
    }

    /// Test placing a stop order for a short position.
    ///
    /// This test function checks the functionality of placing a stop order for a short position by calling the
    /// `can_place_stop_order_short` function with different price scenarios. It verifies that the function returns
    /// the expected results.
    ///
    #[test]
    async fn can_place_stop_order_short_test() {
        reset_for_test().await;

        //Try to place a short order in a lower price (should work);
        let res = can_place_stop_order_short(f64::MAX).await;
        assert!(res, "Can't place the stop order short.");

        //Try to place a short order in a higher price (should not work);
        let res = can_place_stop_order_short(1.0).await;
        assert!(!res, "Can't place the stop order short.");
    }

    /// Test placing a new order with a limit price.
    ///
    /// This test function verifies the functionality of placing a new order with a limit price using the `new_order_limit`
    /// function. It checks whether the order is executed successfully with specific limit prices for both long and short positions.
    ///
    #[test]
    async fn new_order_limit_test() {
        reset_for_test().await;

        //Get current market price
        let res: String = price_ticker("BTCUSDT".to_string()).await.replace('\"', "");
        let market_price = res.parse::<f64>().unwrap();
        let res =
            new_order_limit(market_price * 1.05, &mut 0, true, Some("LONG".to_string())).await;
        assert_eq!(res, "200 OK".to_string());

        let res = new_order_limit(
            market_price * 0.95,
            &mut 0,
            false,
            Some("SHORT".to_string()),
        )
        .await;
        assert_eq!(res, "200 OK".to_string());

        cancel_all_open_orders().await;
    }

    /// Test placing a new order with a market price.
    ///
    /// This test function verifies the functionality of placing a new order with a market price using the `new_order_market`
    /// function. It checks whether the order is executed successfully for both long and short positions.
    ///
    #[test]
    async fn new_order_market_test() {
        reset_for_test().await;

        let res = new_order_market(&mut 0, true, "LONG".to_string()).await;
        assert_eq!(res, "200 OK".to_string());

        let res = new_order_market(&mut 0, false, "SHORT".to_string()).await;
        assert_eq!(res, "200 OK".to_string());

        cancel_all_open_orders().await;
    }

    /// Test getting the stop price of an order.
    ///
    /// This test function checks the functionality of getting the stop price of an order by calling the `get_stop_price`
    /// function. It creates a new order with a specific price, waits for a short duration, and then verifies that the
    /// retrieved stop price matches the expected value.
    ///
    #[test]
    async fn get_stop_price_test() {
        reset_for_test().await;

        let res: String = price_ticker("BTCUSDT".to_string()).await.replace('\"', "");
        let market_price = res.parse::<f64>().unwrap();
        let mut order_id: u64 = 0;
        let res = new_order(
            (market_price * 1.05 * 100.0) / 100.0,
            &mut order_id,
            true,
            false,
            Some("LONG".to_string()),
        )
        .await;

        sleep(Duration::from_secs(1));

        let status = get_stop_price(order_id).await;
        // Arredondar o valor de market_price * 1.05 para a primeira casa decimal
        let expected_status = (market_price * 1.05 * 100.0).trunc() / 100.0 + 1.0;

        // Use assert_approx_eq para verificar se os valores são aproximadamente iguais com uma tolerância de 0.1
        assert_eq!(status, expected_status.to_string());
    }

    /// Test checking the connection to the Binance server.
    ///
    /// This test function verifies the ability to establish a connection to the Binance server by calling the
    /// `test_binance_connection` function. It checks whether the server responds with "200 OK" to indicate a successful
    /// connection.
    ///
    #[test]
    async fn connection_test() {
        let res = test_binance_connection().await;
        assert_eq!(res, "200 OK".to_string());
    }

    /// Test retrieving exchange information.
    ///
    /// This test function checks the functionality of retrieving exchange information by calling the `exchange_info`
    /// function. It verifies that the response contains relevant data such as assets, server time, and symbols.
    ///
    #[test]
    async fn exchange_info_test() {
        let res = exchange_info().await;

        assert!(res.contains("assets"));
        assert!(res.contains("serverTime"));
        assert!(res.contains("symbols"));
        assert_ne!(res, "[]");
    }

    /// Test retrieving the price ticker for a symbol.
    ///
    /// This test function checks the functionality of retrieving the price ticker for a specific symbol by calling
    /// the `price_ticker` function. It verifies that the response contains numeric values, indicating a successful
    /// retrieval of ticker data.
    ///
    #[test]
    async fn price_ticker_test() {
        let res = price_ticker("BTCUSDT".to_string()).await;
        let mut has_num = false;
        for c in res.chars() {
            if c.is_ascii_digit() {
                has_num = true;
            }
        }
        assert!(has_num);
    }

    /// Test retrieving position information.
    // ///
    // /// This test function verifies the functionality of retrieving position information by calling the `position_info`
    // /// function. It checks whether the response indicates a successful operation with "200 OK."
    // ///
    // #[test]
    // async fn position_info_test() {
    //     let pi = position_info().await;
    //     assert_eq!(pi, "200 OK");
    // }

    /// Test activating hedge mode for positions.
    ///
    /// This test function checks the functionality of activating hedge mode for positions by calling the `activate_hedge_mode`
    /// function. It ensures that the operation is successful and handles cases where hedge mode is already active.
    ///
    #[test]
    async fn activate_hedge_mode_test() {
        reset_for_test().await;
        let mut res = activate_hedge_mode().await;

        if res == "200 OK" || res == "E06: No need to change position side." {
            res = "ok".to_string();
        }
        assert_eq!(res, "ok");
    }

    /// Test retrieving information about a specific order.
    ///
    /// This test function checks the functionality of getting information about a specific order by calling the `get_order`
    /// function. It creates a new order, waits briefly, and then verifies that the response contains the order ID.
    ///
    #[test]
    async fn get_order_test() {
        reset_for_test().await;

        activate_hedge_mode().await;
        let mut order_id: u64 = 0;
        let truncated_price: f64 = 200000.0;
        let res = new_order(
            truncated_price,
            &mut order_id,
            false,
            false,
            Some("SHORT".to_string()),
        )
        .await;
        assert_eq!(res, "200 OK".to_string());
        sleep(Duration::from_secs(1));

        let res = get_order(order_id).await;
        assert!(res.contains(&order_id.to_string()));
        close_position(true, Some("SHORT".to_string())).await;
    }

    #[test]
    async fn re_send_request_test() {
        reset_for_test().await;
        let client: reqwest::Client = get_client().await;
        // Cancel the order
        let timestamp = get_timestamp(SystemTime::now()).await;
        let params = format!("symbol=BTCUSDT&timestamp={}&recvWindow=50000", timestamp);
        let signature = get_signature(params.clone()).await;
        let request = format!(
            "{}/fapi/v1/openOrders?{}&signature={}",
            exchange_url().await,
            params.clone(),
            signature.clone()
        );
        // Sending HTTP delete will cancel the order
        let result = match client.get(request.clone()).send().await {
            Ok(response) => response,
            Err(_) => re_send_request(client, "bad request".to_string(), "GET").await,
        };
        assert!(result.status().is_success());
    }
}
