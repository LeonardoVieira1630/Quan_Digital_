// error.rs - Error Handling and Error Constants Mapping

// This file contains the functions and constants related to error handling in the project.
// It provides functions for dealing with errors, as well as mapping possible errors to constants
// to facilitate their reference and usage in other parts of the code.

// Functions:
// - Functions for handling specific errors, such as displaying formatted error messages,
//   fix the problem, etc.

// Constants: (in development)
// - Mapped error constants that can be used in different parts of the code to identify
//   specific errors and facilitate program maintenance and understanding.

// The error.rs file is an essential part of a program that aims to provide robust error handling,
// improving reliability and user experience when dealing with unexpected situations.
// By organizing error-related functions and constants in a single file,
// it is possible to have a clear overview of the functionality and simplify code reuse.

use crate::binance_orders;
use binance_orders::*;
use reqwest::{header, Response, StatusCode};
use serde::Deserialize;
use std::collections::HashMap;

pub const ORDER_WOULD_TRIGGER_IMMEDIATELY: &str = "E01: Order would immediately trigger.";
pub const ERROR_NOT_MAPPED: &str = "E02: Error not mapped.";
pub const ERROR_SERVER_502: &str = "E03: Error 502, exchange server is in trouble.";
pub const ERROR_NOT_VALID_QUANTITY: &str = "E04: The quantity in btc is not valid.";
pub const ERROR_NOTHING_TO_CLOSE: &str = "E05: ReduceOnly Order is rejected.";
pub const NO_NEED_TO_CHANGE_PS: &str = "E06: No need to change position side.";
pub const DNS_ERROR: &str = "E07: Dns error: No such host is known.";
pub const RECVWINDOW_ERROR: &str = "E08: Timestamp for this request is outside of the recvWindow";

#[derive(Debug, Deserialize, Clone)]
pub struct ResultResponseBinance {
    code: i32,
    msg: String,
}

/// Handle errors returned by the Binance API.
///
/// This function is responsible for processing error responses from the Binance API and returning
/// a human-readable error message. It takes two parameters:
///
/// - `result`: The HTTP response containing the error information.
/// - `needed_parameters`: An optional map of parameters needed for handling specific errors.
///
/// If the `result.status()` is not OK (indicating an error response from the API), this function
/// will analyze the error message contained in the response and return an appropriate error message
/// or code. It checks for various error scenarios, including:
///
/// - Orders that would immediately trigger.
/// - 502 Bad Gateway errors.
/// - Errors related to "ReduceOnly" orders.
/// - Errors indicating that there is no need to change the position side.
///
/// If none of the specific error conditions are met, a generic error message is returned.
///
pub async fn error_handler(
    result: Response,
    _needed_parameters: Option<HashMap<String, String>>,
) -> String {
    let result_string = &result.text().await.unwrap();
    //println!(" rs: {}", result_string);
    let result_json: ResultResponseBinance = serde_json::from_str(result_string).unwrap();
    //println!("Order: result text {}", result_string);

    if result_json.msg == "Order would immediately trigger." {
        ORDER_WOULD_TRIGGER_IMMEDIATELY.to_string()
    } else if result_json.msg.contains("502 Bad Gateway") {
        println!("Order: an error occurred: {:?}", result_string);
        ERROR_SERVER_502.to_string()
    } else if result_json.msg.contains("ReduceOnly Order is rejected") {
        ERROR_NOTHING_TO_CLOSE.to_string()
    } else if result_json.msg.contains("No need to change position side") {
        NO_NEED_TO_CHANGE_PS.to_string()
    } else if result_json.msg.contains("No such host is known.") {
        DNS_ERROR.to_string()
    } else if result_json
        .msg
        .contains("Timestamp for this request is outside of the recvWindow.")
    {
        RECVWINDOW_ERROR.to_string()
    } else {
        println!("Order: an error occurred: {:?}", result_string);
        ERROR_NOT_MAPPED.to_string()
    }
}

//Functions tests
#[cfg(test)]
mod tests {
    use super::*;
    use http;
    use reqwest::Response;
    use serde::__private::de::IdentifierDeserializer;
    use tokio::test;

    /// Test handling the "Order would immediately trigger" error.
    ///
    /// This test function simulates an error response with the message "Order would immediately trigger."
    /// It calls the `error_handler` function and verifies that it correctly returns the corresponding error message.
    ///
    #[test]
    async fn test_error_handler_e01() {
        let response_json = r#"{ "code": 1,  "msg": "Order would immediately trigger." }"#;
        let response: Response = Response::from(http::Response::new(response_json));
        let needed_parameters = None;

        assert_eq!(
            error_handler(response, needed_parameters).await,
            "E01: Order would immediately trigger."
        );
    }

    /// Test handling an unmapped error.
    ///
    /// This test function simulates an error response with an unmapped message (e.g., "E02: Error not mapped.").
    /// It calls the `error_handler` function and verifies that it correctly returns the unmapped error message.
    ///
    #[test]
    async fn test_error_handler_e02() {
        let response_json = r#"{ "code": 1,  "msg": "E02: Error not mapped." }"#;
        let response: Response = Response::from(http::Response::new(response_json));
        let needed_parameters = None;

        assert_eq!(
            error_handler(response, needed_parameters).await,
            "E02: Error not mapped."
        );
    }

    /// Test handling a "502 Bad Gateway" error.
    ///
    /// This test function simulates an error response with the message "502 Bad Gateway."
    /// It calls the `error_handler` function and verifies that it correctly returns the mapped error message.
    ///
    #[test]
    async fn test_error_handler_e03() {
        let response_json = r#"{ "code": 1,  "msg": "502 Bad Gateway" }"#;
        let response: Response = Response::from(http::Response::new(response_json));
        let needed_parameters = None;

        assert_eq!(
            error_handler(response, needed_parameters).await,
            "E03: Error 502, exchange server is in trouble."
        );
    }

    /// Test handling a "ReduceOnly Order is rejected" error.
    ///
    /// This test function simulates an error response with the message "E05: ReduceOnly Order is rejected."
    /// It calls the `error_handler` function and verifies that it correctly returns the mapped error message.
    ///
    #[test]
    async fn test_error_handler_e05() {
        let response_json = r#"{ "code": 1,  "msg": "E05: ReduceOnly Order is rejected." }"#;
        let response: Response = Response::from(http::Response::new(response_json));
        let needed_parameters = None;

        assert_eq!(
            error_handler(response, needed_parameters).await,
            "E05: ReduceOnly Order is rejected."
        );
    }
}
