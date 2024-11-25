# WPILog Parser

A heavily-optimized parser for WPILib's `.wpilog` data format.

## Features

By default, this library will prefer performance to safety with extensive use of debug_assert.
Enabling the `safe` feature will ensure that the program will use 100% safe code and won't panic, at the cost of performance.
