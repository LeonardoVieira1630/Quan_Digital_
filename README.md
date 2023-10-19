# Quan Digital Automated Trading Strategies 🦏 💰

Welcome to the Quan Digital Trading repository. Quan Digital is a company specializing in the development of advanced automated trading strategies for the financial industry. While our proprietary trading strategies remain confidential, this repository contains a collection of auxiliary code and tools that support our overall trading infrastructure.

At Quan Digital, we employ a meticulous approach to trading strategy development, utilizing extensive backtesting to ensure their effectiveness and reliability in dynamic market conditions. Each strategy is thoughtfully implemented in Rust, a high-performance programming language known for its efficiency and robustness.

To address the absence of an official Software Development Kit (SDK) for Rust-based systems in the context of the Binance Futures Exchange, our team has developed a bespoke infrastructure for communication. This infrastructure not only enables the execution of our strategies but also guarantees precision and security in trade execution.

This repository encompasses a valuable collection of code, tools, and resources that have been diligently maintained and continuously improved by our expert team. Although it does not include our proprietary trading strategies, it provides a glimpse into the supportive ecosystem that powers our trading operations.

# About the Codes

In this section, we will provide an overview of the codes contained in this repository. The codes made available here are an essential part of our trading infrastructure, supporting Quan Digital's automated strategies. Each code plays a pivotal role in facilitating and enhancing our trading operations. We will now provide detailed information about each of these codes and how they contribute to the functioning of our system.

## Binance Orders Functions (binance_orders.rs)

The `binance_orders.rs` file serves as a central hub for essential functions related to order placement, execution, and the retrieval of order information in a Binance trading bot project. This file plays a pivotal role in the functionality of the trading bot, enabling interactions with the Binance exchange's order placement and retrieval system.

### Some Functions and Features

1. **`new_order` Function:**

   - This function facilitates the placement of a new order in the Binance exchange.
   - Parameters include the order price, order ID, order type (buy/sell), and the ability to specify whether the order is "reduce-only."
   - It also supports position side differentiation for "LONG" and "SHORT" positions.

2. **`close_position` Function:**
   - The `close_position` function enables the closure of positions on the Binance exchange.
   - Users can specify whether they wish to execute a market buy or market sell order.
   - The function provides options for specifying the position side to close ("LONG," "SHORT," or "BOTH").
3. **Test Functions:**
   - The file includes test functions such as `get_order_test` and `re_send_request_test` to verify the functionality and robustness of the order-related functions.
   - These tests simulate real-world scenarios to ensure proper execution and handling of potential errors.

By organizing these functions in a separate file, `binance_orders.rs` promotes code modularity and maintainability. This modular approach makes it easier to manage and extend the bot's order-related capabilities, contributing to a more robust and efficient trading system for Binance.

## Error Handling and Constants Mapping (error.rs)

The `error.rs` file is a critical component of our project, dedicated to handling errors and mapping error constants for improved error management. It centralizes functions related to error processing and maintains constants that represent possible errors. This organization simplifies the identification and handling of specific errors throughout the codebase.

### Key Functions and Constants

1. **`error_handler` Function:**

   - The `error_handler` function is responsible for processing error responses returned by the Binance API. It returns human-readable error messages and error codes based on the response from the API.
   - This function analyzes the HTTP response and checks for various error scenarios, such as orders that would immediately trigger, 502 Bad Gateway errors, errors related to "ReduceOnly" orders, and more.
   - If none of the specific error conditions are met, it returns a generic error message.

2. **Error Constants:**

   - The `error.rs` file defines and maintains a set of error constants (e.g., `ORDER_WOULD_TRIGGER_IMMEDIATELY`, `ERROR_NOT_MAPPED`, `ERROR_SERVER_502`, etc.).
   - These constants facilitate the identification of specific errors throughout the codebase, making it easier to understand and maintain the program.

By centralizing error-related functions and constants in this file, we enhance the project's error handling and error message clarity, ultimately improving reliability and the user experience when dealing with unexpected situations. The organization of error handling in one location simplifies code reuse and maintenance efforts.

## Candle Functions for Fetching Candlestick Data (get_candles.rs)

The `get_candles.rs` file provides essential functions to interact with the Binance API and retrieve candlestick information for a specified trading pair. These functions are vital for performing technical analysis, backtesting trading strategies, and executing trading operations.

### Prerequisites

Before using these functions, make sure you have a valid Binance API key and secret key to authenticate your API requests. It's crucial to handle API authentication securely, following best practices and protecting sensitive information.

Refer to the Binance API documentation for detailed information on the required parameters, possible responses, and any additional considerations for retrieving candlestick data.

### Code Highlights

This file utilizes various libraries and dependencies, including:

- `async_recursion` for handling asynchronous requests.
- `chrono` for working with timestamps.
- `reqwest` for making HTTP requests to the Binance API.
- `hmac` and `sha2` for generating API signatures.
- `std` libraries for standard Rust functionality.

### Key Functions

1. **`get_candle_last_min` Function:**

   This function fetches the closing price of the last one-minute closed candle for the BTCUSDT trading pair. It retrieves the most recent completed candlestick data.

   - Returns:
     - `Ok(f64)`: The closing price of the last closed candle, greater than 0.0.
     - `Err(String)`: An error message if the request fails.

2. **`get_some_1m_candle` Function:**

   This function retrieves the closing prices of the last 'quantity' one-minute candles for the BTCUSDT trading pair.

   - Arguments:

     - `quantity`: The number of one-minute candles to retrieve.

   - Returns:
     - `Ok(BTreeMap<i64, f64>)`: A `BTreeMap` where the key is the timestamp, and the value is the closing price.
     - `Err(String)`: An error message if the request fails.

3. **`get_candle_info` Function:**

   This function retrieves the closing prices of 'quantity' candles for a specified trading pair and interval. It calculates the number of one-minute candles required based on the interval.

   - Arguments:

     - `quantity`: The number of candles to retrieve.
     - `symbol`: The trading pair symbol (e.g., "BTCUSDT").
     - `interval`: The candle interval as a string (e.g., "1h").

   - Returns:
     - `Ok(Vec<f64>)`: A vector containing the closing prices of the retrieved candles.
     - `Err(String)`: An error message if the request fails.

4. **`get_some_candles_from_binance` Function:**

   This function fetches the closing prices of 'quantity' candles for a specified trading pair and interval, directly from Binance.

   - Arguments:

     - `quantity`: The number of candles to retrieve.
     - `interval`: The candle interval as a string (e.g., "1h").

   - Returns:
     - `Ok(BTreeMap<i64, f64>)`: A `BTreeMap` where the key is the timestamp, and the value is the closing price.
     - `Err(String)`: An error message if the request fails.

5. **`build_candle_w_1hr_close_price` Function:**

   This function constructs candles with closing prices from one-hour candles for a specified quantity and interval.

   - Arguments:

     - `quantity`: The number of candles to build.
     - `symbol`: The trading pair symbol (e.g., "BTCUSDT").
     - `interval`: The candle interval as a string (e.g., "3h").

   - Returns:
     - `Ok(Vec<f64>)`: A vector containing the closing prices of the built candles.
     - `Err(String)`: An error message if the request fails.

### Function Tests

The `get_candles.rs` file includes a test module with various test functions to ensure the correct functionality of the provided functions. You can run these tests to verify that the functions work as expected.

## Additional Code Resources

This repository includes a variety of code files related to different functionalities and features of the project. While this README provides an overview of specific code files, it's important to note that there are other code files not covered in detail here.

"Additional files are accessible within this repository; however, it's important to note that the majority of the Quan Digital files are kept an other private repository and won't be shared publicly. However, the available resources are still valuable assets that can be explored, studied, and employed according to your project requirements. Feel free to leverage these files as needed for your work."

Remember that you can clone this repository, experiment with the code, and adapt it to your specific requirements. If you have any questions or need clarification on any part of the code or related files, don't hesitate to get in touch.

<div align="center">
  <img src="quan_name.png" alt="Logo da Quan Digital" width="200" height="100">
</div>
