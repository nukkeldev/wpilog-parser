# Design

## References
- [DataLog Spec](https://github.com/wpilibsuite/allwpilib/blob/main/wpiutil/doc/datalog.adoc)

## Glossary
- *UB* - Undefined Behavior
- *CR* - Control Record
- *SCR* - Start *CR*
- *FCR* - Finish *CR*

## Assumptions
- All utf8 encoded strings are well-formed.

## Non-Spec Requirements:
- No duplicate entry names.
    - NT entries are identified by name, so it makes no sense to have them conflict. The only scenario where this would make (technical) sense would be to change the type of a pre-existing entry, but even then, you're doing something wrong.

## Non-requirements
- Modification
    - Parsed logs are read-only. Everything is zero-copy (when applicable) to be as efficient as possible. Parsed logs may be converted to owned ones in order to modify them.

## *UB* Definitions
- Consecutive *SCR*s, without intermediary *FCR*s, to the the same entry id is *UB*. This program will treat each consecutive *SCR* as an *FCR* for the prior entry, in addition to a new entry.
    - See end of paragraph 2 in the `Design` section of the spec.

## Parsed Usage Requirements:
- Indexing by Entry Name
- Export to Polars DataFrame

## Parsing
- Two-pass
    1. CRs collected into initial `entries` structure
    2. Records associated with corresponding `Entry<'_>`

### Algorithm
> Number Parsing (NP) = try_into -> from_le_bytes
> String Parsing (SP) = NP -> len @ u32 -> str::from_utf8[4..len+4]

1. Given `&'a [u8] data`
2. Verify ✨magic✨ (b"WPILOG")
3. Parse `version number` (NP -> u16) and `metadata` (SP)
4. Parse records until EOF (First pass)
    1. Parse header length bitfield 1-bit zero, 3-bits timestamp length - 1, 2-bits payload size length - 1, and 2 bits entry id length - 1
    2. Parse `entry id`, `payload size`, `timestamp`, and `payload`
    3. If `entry id` is not `0` -> continue
    4. Match `payload[0]`
        1. If `0`, 

## Structure
struct WPILog<'a> {
    version: u16, // The major and minor version numbers for the parsed log
    metadata: &'a str, // Arbitrary metadata
    entries: HashMap<&'a str, Entry<'a>>, // Name correlated entries 
}