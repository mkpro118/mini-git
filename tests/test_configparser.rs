use mini_git::configparser::*;

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
    }
}
