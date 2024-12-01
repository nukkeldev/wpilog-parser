# WPILib DataLog Parser

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

> This crate is still in-development. Lack of documentation or partially inefficient code will be fixed shortly.

An efficient parser for WPILib's DataLog format (used with their `.wpilog` logs)

## Features

- Parsing a `.wpilog` file into a read-only `DataLog` struct

## Non-features

- Verifying the correctness of a `.wpilog` file
  - It is assumed all files are loaded from a trusted, correct source. As such, and in general with binary formats, malformed or corrupt data is easier corrected with regeneration rather than diagnosing/correcting errors in the file (a corrupt file will likely not given correct data regardless).
- Anything beyond parsing
  - This crate makes no assumptions on how the data will be used and parses all the data necessary for this file to be reconstructed.

## References

[WPILib Data Log File Format Specification, Version 1.0](https://github.com/wpilibsuite/allwpilib/blob/main/wpiutil/doc/datalog.adoc)
