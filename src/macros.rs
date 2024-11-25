/// Given a slice, collects the supplied indicies into an array.
///
/// For example,
/// ```rust,ignore
///     let data = &[1, 2, 3, 4];
///     let extracted = extract_from_slice!(data, 0, 3); // => [data[0], data[3]]
///     assert_eq!(extracted, [1, 4])
/// ```
#[macro_export]
macro_rules! extract_from_slice {
    ($slice:expr, $($idx:expr),*) => {
        [$($slice[$idx]),*]
    };
}

/// Depending on the "safe" feature, returns an `Err` or `debug_assert`s the condition (using the error message).
#[macro_export]
macro_rules! err_if_safe {
    ($condition:expr, $err:expr) => {
        #[cfg(feature = "safe")]
        {
            if $condition {
                return Err($err);
            }
        }
        #[cfg(not(feature = "safe"))]
        {
            debug_assert!($condition, "{}", $err);
        }
    };
}
