use std::borrow::Cow;
use std::path::{Path, PathBuf};

use rocket::request::FromParam;
use rocket::http::RawStr;
use rand::{self, Rng};

/// Table to retrieve base62 values from.
const BASE62: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

/// A _probably_ unique paste ID.
#[derive(UriDisplayPath)]
pub struct PasteID<'a>(Cow<'a, str>);

impl PasteID<'_> {
    /// Generate a _probably_ unique ID with `size` characters. For readability,
    /// the characters used are from the sets [0-9], [A-Z], [a-z]. The
    /// probability of a collision depends on the value of `size` and the number
    /// of IDs generated thus far.
    pub fn new(size: usize) -> PasteID<'static> {
        let mut id = String::with_capacity(size);
        let mut rng = rand::thread_rng();
        for _ in 0..size {
            id.push(BASE62[rng.gen::<usize>() % 62] as char);
        }

        PasteID(Cow::Owned(id))
    }

    pub fn file_path(&self) -> PathBuf {
        Path::new("upload").join(self.0.as_ref())
    }
}

/// Returns an instance of `PasteID` if the path segment is a valid ID.
/// Otherwise returns the invalid ID as the `Err` value.
impl<'a> FromParam<'a> for PasteID<'a> {
    type Error = &'a RawStr;

    fn from_param(param: &'a RawStr) -> Result<Self, Self::Error> {
        match param.as_str().chars().all(|c| c.is_ascii_alphanumeric()) {
            true => Ok(PasteID(Cow::Borrowed(param.as_str()))),
            false => Err(param)
        }
    }
}
