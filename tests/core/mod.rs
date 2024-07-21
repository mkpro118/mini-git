pub mod test_hash_object;
pub mod test_init;

#[macro_export]
macro_rules! make_namespaces_from {
    ($maker:ident) => {
        fn make_namespaces<'a>(
            args: &'a [&[&'a str]],
        ) -> impl Iterator<Item = mini_git::utils::argparse::Namespace> + 'a {
            let mut parser = $maker();
            parser.compile();

            args.iter().flat_map(move |&x| parser.parse_args(x))
        }
    };
}

pub static TEST_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[macro_export]
macro_rules! with_test_mutex {
    ($body:block) => {
        if let Ok(_) = crate::core::TEST_MUTEX.lock() {
            $body
        } else {
            panic!("Test Mutex failed!");
        }
    };
}
