use crate::request::Request;
use crate::data::{Data, FromData, Outcome};
use crate::http::{RawStr, ext::IntoOwned};
use crate::form::parser::{Parser, RawStrParser, Buffer};
use crate::form::prelude::*;

/// A data guard for parsing [`FromForm`] types strictly.
///
/// This type implements the [`FromTransformedData`] trait. It provides a
/// generic means to parse arbitrary structures from incoming form data.
///
/// # Strictness
///
/// A `Form<T>` will parse successfully from an incoming form only if the form
/// contains the exact set of fields in `T`. Said another way, a `Form<T>` will
/// error on missing and/or extra fields. For instance, if an incoming form
/// contains the fields "a", "b", and "c" while `T` only contains "a" and "c",
/// the form _will not_ parse as `Form<T>`. If you would like to admit extra
/// fields without error, see [`LenientForm`](crate::request::LenientForm).
///
/// # Usage
///
/// This type can be used with any type that implements the `FromForm` trait.
/// The trait can be automatically derived; see the [`FromForm`] documentation
/// for more information on deriving or implementing the trait.
///
/// Because `Form` implements `FromTransformedData`, it can be used directly as a target of
/// the `data = "<param>"` route parameter as long as its generic type
/// implements the `FromForm` trait:
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// use rocket::request::Form;
/// use rocket::http::RawStr;
///
/// #[derive(FromForm)]
/// struct UserInput<'f> {
///     // The raw, undecoded value. You _probably_ want `String` instead.
///     value: &'f RawStr
/// }
///
/// #[post("/submit", data = "<user_input>")]
/// fn submit_task(user_input: Form<UserInput>) -> String {
///     format!("Your value: {}", user_input.value)
/// }
/// # fn main() {  }
/// ```
///
/// A type of `Form<T>` automatically dereferences into an `&T` or `&mut T`,
/// though you can also transform a `Form<T>` into a `T` by calling
/// [`into_inner()`](Form::into_inner()). Thanks to automatic dereferencing, you
/// can access fields of `T` transparently through a `Form<T>`, as seen above
/// with `user_input.value`.
///
/// For posterity, the owned analog of the `UserInput` type above is:
///
/// ```rust
/// struct OwnedUserInput {
///     // The decoded value. You _probably_ want this.
///     value: String
/// }
/// ```
///
/// A handler that handles a form of this type can similarly by written:
///
/// ```rust
/// # #![allow(deprecated, unused_attributes)]
/// # #[macro_use] extern crate rocket;
/// # use rocket::request::Form;
/// # #[derive(FromForm)]
/// # struct OwnedUserInput {
/// #     value: String
/// # }
/// #[post("/submit", data = "<user_input>")]
/// fn submit_task(user_input: Form<OwnedUserInput>) -> String {
///     format!("Your value: {}", user_input.value)
/// }
/// # fn main() {  }
/// ```
///
/// Note that no lifetime annotations are required in either case.
///
/// ## `&RawStr` vs. `String`
///
/// Whether you should use a `&RawStr` or `String` in your `FromForm` type
/// depends on your use case. The primary question to answer is: _Can the input
/// contain characters that must be URL encoded?_ Note that this includes common
/// characters such as spaces. If so, then you must use `String`, whose
/// [`FromFormValue`](crate::request::FromFormValue) implementation automatically URL
/// decodes the value. Because the `&RawStr` references will refer directly to
/// the underlying form data, they will be raw and URL encoded.
///
/// If it is known that string values will not contain URL encoded characters,
/// or you wish to handle decoding and validation yourself, using `&RawStr` will
/// result in fewer allocation and is thus preferred.
///
/// ## Incoming Data Limits
///
/// The default size limit for incoming form data is 32KiB. Setting a limit
/// protects your application from denial of service (DOS) attacks and from
/// resource exhaustion through high memory consumption. The limit can be
/// increased by setting the `limits.forms` configuration parameter. For
/// instance, to increase the forms limit to 512KiB for all environments, you
/// may add the following to your `Rocket.toml`:
///
/// ```toml
/// [global.limits]
/// forms = 524288
/// ```
#[derive(Debug)]
pub struct Form<T>(T);

impl<T> Form<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl Form<()> {
    /// `string` must represent a decoded string.
    pub fn values(string: &str) -> impl Iterator<Item = ValueField<'_>> {
        // WHATWG URL Living Standard 5.1 steps 1, 2, 3.1 - 3.3.
        string.split('&')
            .filter(|s| !s.is_empty())
            .map(ValueField::parse)
    }
}

impl<'r, T: FromForm<'r>> Form<T> {
    /// `string` must represent a decoded string.
    pub fn parse(string: &'r str) -> Result<'r, T> {
        // WHATWG URL Living Standard 5.1 steps 1, 2, 3.1 - 3.3.
        let mut ctxt = T::init(Options::Lenient);
        Form::values(string).for_each(|f| T::push_value(&mut ctxt, f));
        T::finalize(ctxt)
    }
}

impl<T: for<'a> FromForm<'a> + 'static> Form<T> {
    /// `string` must represent an undecoded string.
    pub fn parse_encoded(string: &RawStr) -> Result<'static, T> {
        let buffer = Buffer::new();
        let mut ctxt = T::init(Options::Lenient);
        for field in RawStrParser::new(&buffer, string) {
            T::push_value(&mut ctxt, field)
        }

        T::finalize(ctxt).map_err(|e| e.into_owned())
    }
}

impl<T> std::ops::Deref for Form<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> std::ops::DerefMut for Form<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[crate::async_trait]
impl<'r, T: FromForm<'r>> FromData<'r> for Form<T> {
    type Error = Errors<'r>;

    async fn from_data(req: &'r Request<'_>, data: Data) -> Outcome<Self, Self::Error> {
        use either::Either;

        let mut parser = try_outcome!(Parser::new(req, data).await);
        let mut context = T::init(Options::Lenient);
        while let Some(field) = parser.next().await {
            match field {
                Ok(Either::Left(value)) => T::push_value(&mut context, value),
                Ok(Either::Right(data)) => T::push_data(&mut context, data).await,
                Err(e) => T::push_error(&mut context, e),
            }
        }

        match T::finalize(context) {
            Ok(value) => Outcome::Success(Form(value)),
            Err(e) => Outcome::Failure((e.status(), e)),
        }
    }
}
