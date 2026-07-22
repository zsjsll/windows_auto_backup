#[cfg(not(feature = "dbg"))]
#[macro_export]
macro_rules! dbg {
    ($val:expr $(,)?) => {{
        match $val {
            tmp => tmp,
        }
    }};

    ($($val:expr),+ $(,)?) => {{
        match ($($val),+) {
            tmp => tmp,
        }
    }};
}

#[cfg(not(feature = "dbg"))]
#[macro_export]
macro_rules! println {
    ($val:expr $(,)?) => {{
        match $val {
            tmp => tmp,
        }
    }};

    ($($val:expr),+ $(,)?) => {{
        match ($($val),+) {
            tmp => tmp,
        }
    }};
}
