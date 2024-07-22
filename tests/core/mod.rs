pub mod test_cat_file;
pub mod test_hash_object;
pub mod test_init;
pub mod test_ls_tree;

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
