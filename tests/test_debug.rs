#[cfg(test)]
mod tests {
    use mini_git::datetime::*;

    #[test]
    fn test_datetime_to_str() {
        let timestamp = 1704153600; // > January 1, 2024 in all timezones
        let dt = DateTime::from_timestamp(timestamp);
        let str_repr = dt.to_str();
        dbg!(&dt);
        dbg!(&str_repr);

        // Check that the string contains the expected date
        assert!(str_repr.contains("2024"));
        assert!(str_repr.contains("Jan"));
    }

    #[test]
    fn test_datetime_debug_impl() {
        let dt = DateTime::from_timestamp(1609459200);
        let debug_str = format!("{:?}", dt);
        assert!(debug_str.contains("time:"));
        assert!(debug_str.contains("tz:"));
    }
}