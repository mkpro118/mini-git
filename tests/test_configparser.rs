use mini_git::configparser::*;
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
            let ini_content = r#"
[Section1]
key1 = value1
key2 = value2

[Section2]
key3 = value3
"#;
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
            let non_existent_path = env::temp_dir().join("nonexistent_file.ini");
            let _ = ConfigParser::from(non_existent_path.as_path());
        }

        #[test]
        fn test_from_iter_string() {
            let lines = vec![
                "".to_string(),
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

            let parser = ConfigParser::from_iter(lines.into_iter());

            assert_eq!(parser["Section1"]["key1"], "value1");
            assert_eq!(parser["Section1"]["key2"], "value2");
            assert_eq!(parser["Section2"]["key3"], "value3");
        }

        #[test]
        fn test_global_section() {
            let ini_content = r#"
global_key = global_value
[Section1]
key1 = value1
"#;
            let parser = ConfigParser::from(ini_content);

            assert_eq!(parser[""]["global_key"], "global_value");
            assert_eq!(parser["Section1"]["key1"], "value1");
        }

        #[test]
        fn test_empty_value() {
            let ini_content = r#"
[Section1]
key1 =
key2 = value2
"#;
            let parser = ConfigParser::from(ini_content);

            assert_eq!(parser["Section1"]["key1"], "");
            assert_eq!(parser["Section1"]["key2"], "value2");
        }
    }
}
