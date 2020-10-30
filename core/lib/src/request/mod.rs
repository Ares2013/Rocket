//! Types and traits for request parsing and handling.

mod request;
mod from_param;
mod from_request;

#[cfg(test)]
mod tests;

pub use self::request::Request;
pub use self::from_request::{FromRequest, Outcome};
pub use self::from_param::{FromParam, FromSegments};

#[doc(inline)]
pub use crate::response::flash::FlashMessage;

#[macro_export]
#[doc(hidden)]
macro_rules! local_cache {
    ($req:expr, $v:expr) => ({
        struct Local<T>(T);
        &$req.local_cache(move || Local($v)).0
    })
}

pub use local_cache;
