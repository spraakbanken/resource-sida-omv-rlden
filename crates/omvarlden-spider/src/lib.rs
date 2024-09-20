mod error;
pub mod item;
pub mod omvarlden;

pub use error::Error;
pub static APP_USER_AGENT: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
    ";",
    "SpråkbankenTextBot/1.0"
);
