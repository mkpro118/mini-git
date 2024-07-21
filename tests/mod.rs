pub mod core;
pub mod utils;

pub static TEST_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[macro_export]
macro_rules! with_test_mutex {
    ($body:block) => {
        if let Ok(_) = $crate::TEST_MUTEX.lock() {
            $body
        } else {
            panic!("Test Mutex failed!");
        }
    };
}
