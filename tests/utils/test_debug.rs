#[cfg(test)]
mod tests {
    use mini_git::utils::datetime::*;

    #[test]
    fn test_datetime_to_str() {
        let timestamp = 1_704_153_600; // > January 1, 2024 in all timezones
        let dt = DateTime::from_timestamp(timestamp);
        let str_repr = dt.to_str();

        // Check that the string contains the expected date
        assert!(str_repr.contains("2024"));
        assert!(str_repr.contains("Jan"));
    }

    #[test]
    fn test_datetime_debug_impl() {
        let dt = DateTime::from_timestamp(1_609_459_200);
        let debug_str = format!("{dt:?}");
        assert!(debug_str.contains("time:"));
        assert!(debug_str.contains("tz:"));
    }
}
