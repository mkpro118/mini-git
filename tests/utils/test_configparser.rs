use mini_git::utils::configparser::*;
use std::env;
use std::fs::File;
use std::io::Write;

#[cfg(test)]
mod tests {
    use super::*;

    mod config_section_tests {
        use super::*;

        #[test]
        fn test_add_config() {
            let mut section = ConfigSection::new();
            section.add_config("key1", "value1");
            assert_eq!(section["key1"], "value1");
        }

        #[test]
        fn test_index() {
            let mut section = ConfigSection::new();
            section.add_config("key1", "value1");
            assert_eq!(section["key1"], "value1");
        }

        #[test]
        fn test_index_mut() {
            let mut section = ConfigSection::new();
            section["key1"] = "value1".to_string();
            assert_eq!(section["key1"], "value1");
        }

        #[test]
        fn test_index_mut_new_key() {
            let mut section = ConfigSection::new();
            section["new_key"] = "new_value".to_string();
            assert_eq!(section["new_key"], "new_value");
        }

        fn setup_config_section() -> ConfigSection {
            let mut section = ConfigSection::new();
            section.add_config("int_value", "42");
            section.add_config("negative_int", "-10");
            section.add_config("float_value", "3.14");
            section.add_config("negative_float", "-2.5");
            section.add_config("bool_true", "true");
            section.add_config("bool_false", "false");
            section.add_config("bool_yes", "yes");
            section.add_config("bool_no", "no");
            section.add_config("bool_on", "on");
            section.add_config("bool_off", "off");
            section.add_config("bool_1", "1");
            section.add_config("bool_0", "0");
            section.add_config("invalid_value", "not_a_number");
            section
        }

        #[test]
        fn test_get_int() {
            let section = setup_config_section();

            assert_eq!(section.get_int("int_value"), Some(42));
            assert_eq!(section.get_int("negative_int"), Some(-10));
            assert_eq!(section.get_int("non_existent"), None);
        }

        #[test]
        #[should_panic(expected = "ParseIntError")]
        fn test_get_int_from_float() {
            let section = setup_config_section();

            assert_eq!(section.get_int("float_value"), Some(3));
        }

        #[test]
        #[should_panic(expected = "Should be parsed as float")]
        fn test_get_int_invalid() {
            let section = setup_config_section();
            let _ = section.get_int("invalid_value");
        }

        #[test]
        #[allow(clippy::approx_constant)]
        fn test_get_float() {
            let section = setup_config_section();

            assert_eq!(section.get_float("float_value"), Some(3.14));
            assert_eq!(section.get_float("negative_float"), Some(-2.5));
            assert_eq!(section.get_float("non_existent"), None);
        }

        #[test]
        fn test_get_float_from_int() {
            let section = setup_config_section();

            assert_eq!(section.get_float("int_value"), Some(42.0));
        }

        #[test]
        #[should_panic(expected = "Should be parsed as float")]
        fn test_get_float_invalid() {
            let section = setup_config_section();
            let _ = section.get_float("invalid_value");
        }

        #[test]
        fn test_get_bool() {
            let section = setup_config_section();

            assert_eq!(section.get_bool("bool_true"), Some(true));
            assert_eq!(section.get_bool("bool_false"), Some(false));
            assert_eq!(section.get_bool("bool_yes"), Some(true));
            assert_eq!(section.get_bool("bool_no"), Some(false));
            assert_eq!(section.get_bool("bool_on"), Some(true));
            assert_eq!(section.get_bool("bool_off"), Some(false));
            assert_eq!(section.get_bool("bool_1"), Some(true));
            assert_eq!(section.get_bool("bool_0"), Some(false));
            assert_eq!(section.get_bool("non_existent"), None);
        }

        #[test]
        fn test_get_bool_case_insensitive() {
            let mut section = ConfigSection::new();
            section.add_config("upper_true", "TRUE");
            section.add_config("mixed_false", "FaLsE");

            assert_eq!(section.get_bool("upper_true"), Some(true));
            assert_eq!(section.get_bool("mixed_false"), Some(false));
        }

        #[test]
        fn test_get_bool_invalid() {
            let section = setup_config_section();

            assert_eq!(section.get_bool("invalid_value"), None);
            assert_eq!(section.get_bool("int_value"), None);
            assert_eq!(section.get_bool("float_value"), None);
        }
    }

    mod config_parser_tests {
        use super::*;

        #[test]
        fn test_add_config() {
            let mut parser = ConfigParser::new();
            parser.add_config("section1", "key1", "value1");
            assert_eq!(parser["section1"]["key1"], "value1");
        }

        #[test]
        fn test_index() {
            let mut parser = ConfigParser::new();
            parser.add_section("section1");
            parser["section1"].add_config("key1", "value1");
            assert_eq!(parser["section1"]["key1"], "value1");
        }

        #[test]
        fn test_index_mut() {
            let mut parser = ConfigParser::new();
            parser["section1"]["key1"] = "value1".to_string();
            assert_eq!(parser["section1"]["key1"], "value1");
        }

        #[test]
        fn test_index_mut_new_section() {
            let mut parser = ConfigParser::new();
            parser["new_section"]["new_key"] = "new_value".to_string();
            assert_eq!(parser["new_section"]["new_key"], "new_value");
        }

        #[test]
        fn test_multiple_sections_and_configs() {
            let mut parser = ConfigParser::new();
            parser.add_config("section1", "key1", "value1");
            parser.add_config("section1", "key2", "value2");
            parser.add_config("section2", "key3", "value3");

            assert_eq!(parser["section1"]["key1"], "value1");
            assert_eq!(parser["section1"]["key2"], "value2");
            assert_eq!(parser["section2"]["key3"], "value3");
        }

        #[test]
        fn test_from_str_ini_content() {
            let ini_content = r"
[Section1]
key1 = value1
key2 = value2

[Section2]
key3 = value3
";
            let parser = ConfigParser::from(ini_content);

            assert_eq!(parser["Section1"]["key1"], "value1");
            assert_eq!(parser["Section1"]["key2"], "value2");
            assert_eq!(parser["Section2"]["key3"], "value3");
        }

        #[test]
        fn test_from_path() {
            let temp_dir = env::temp_dir();
            let file_path = temp_dir.join("test_config.ini");
            let mut file = File::create(&file_path).unwrap();
            writeln!(file, "[Section1]").unwrap();
            writeln!(file, "key1 = value1").unwrap();
            writeln!(file, "[Section2]").unwrap();
            writeln!(file, "key2 = value2").unwrap();
            file.flush().unwrap();

            let parser = ConfigParser::from(file_path.as_path());

            assert_eq!(parser["Section1"]["key1"], "value1");
            assert_eq!(parser["Section2"]["key2"], "value2");

            // Clean up
            std::fs::remove_file(file_path).unwrap();
        }

        #[test]
        #[should_panic(expected = "File")]
        fn test_from_nonexistent_path() {
            let non_existent_path =
                env::temp_dir().join("nonexistent_file.ini");
            let _ = ConfigParser::from(non_existent_path.as_path());
        }

        #[test]
        fn test_from_iter_string() {
            let lines = vec![
                String::new(),
                "; This is a comment".to_string(),
                "[Section1]".to_string(),
                "key1 = value1".to_string(),
                "key2 = value2".to_string(),
                "[Section2]".to_string(),
                "key3 = value3".to_string(),
            ];

            let parser = ConfigParser::from_iter(lines);

            assert_eq!(parser["Section1"]["key1"], "value1");
            assert_eq!(parser["Section1"]["key2"], "value2");
            assert_eq!(parser["Section2"]["key3"], "value3");
        }

        #[test]
        fn test_from_iter_str_slice() {
            let lines = [
                "",
                "; This is a comment",
                "[Section1]",
                "key1 = value1",
                "key2 = value2",
                "[Section2]",
                "key3 = value3",
            ];

            let parser = ConfigParser::from_iter(lines);

            assert_eq!(parser["Section1"]["key1"], "value1");
            assert_eq!(parser["Section1"]["key2"], "value2");
            assert_eq!(parser["Section2"]["key3"], "value3");
        }

        #[test]
        fn test_global_section() {
            let ini_content = r"
global_key = global_value
[Section1]
key1 = value1
";
            let parser = ConfigParser::from(ini_content);

            assert_eq!(parser[""]["global_key"], "global_value");
            assert_eq!(parser["Section1"]["key1"], "value1");
        }

        #[test]
        fn test_empty_value() {
            let ini_content = r"
[Section1]
key1 =
key2 = value2
";
            let parser = ConfigParser::from(ini_content);

            assert_eq!(parser["Section1"]["key1"], "");
            assert_eq!(parser["Section1"]["key2"], "value2");
        }
    }
}
