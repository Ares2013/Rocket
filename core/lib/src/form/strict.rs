use crate::form::prelude::*;

use crate::http::uri::{Query, FromUriParam};

/// A form guard for parsing form types strictly.
///
/// This type implements the [`FromForm`] trait and thus can be used as a
/// generic parameter to the [`Form`] data guard: `Form<Strict<T>>`, where `T`
/// implements `FromForm`. Unlike using `Form` directly, this type uses a
/// _strict_ parsing strategy: forms that contains a superset of the expected
/// fields (i.e, extra fields) will fail to parse.
///
/// # Strictness
///
/// A `Form<Strict<T>>` will parse successfully from an incoming form only if
/// the form contains the exact set of fields in `T`. Said another way, a
/// `Form<T>` will error on missing and/or extra fields. For instance, if an
/// incoming form contains the fields "a", "b", and "c" while `T` only contains
/// "a" and "c", the form _will not_ parse as `Form<T>`.
///
/// # Usage
///
/// `Strict<T>` implements `FromForm` as long as `T` implements `FromForm`. As
/// such, `Form<Strict<T>>` is a data guard:
///
/// ```rust
/// # #[macro_use] extern crate rocket;
/// use rocket::form::Strict;
///
/// #[derive(FromForm)]
/// struct UserInput {
///     value: String
/// }
///
/// #[post("/submit", data = "<user_input>")]
/// fn submit_task(user_input: Form<Strict<UserInput>>) -> String {
///     format!("Your value: {}", user_input.value)
/// }
/// # fn main() {  }
/// ```
#[derive(Debug)]
pub struct Strict<T>(T);

impl<T> Strict<T> {
    /// Consumes `self` and returns the parsed value.
    ///
    /// # Example
    ///
    /// ```rust
    /// # #[macro_use] extern crate rocket;
    /// use rocket::request::LenientForm;
    ///
    /// #[derive(FromForm)]
    /// struct MyForm {
    ///     field: String,
    /// }
    ///
    /// #[post("/submit", data = "<form>")]
    /// fn submit(form: LenientForm<MyForm>) -> String {
    ///     form.into_inner().field
    /// }
    /// ```
    pub fn into_inner(self) -> T {
        self.0
    }
}

#[crate::async_trait]
impl<'v, T: FromForm<'v>> FromForm<'v> for Strict<T> {
    type Context = T::Context;

    #[inline(always)]
    fn init(opts: Options) -> Self::Context {
        T::init(Options { strict: true, ..opts })
    }

    #[inline(always)]
    fn push_value(ctxt: &mut Self::Context, field: ValueField<'v>) {
        T::push_value(ctxt, field)
    }

    #[inline(always)]
    async fn push_data(ctxt: &mut Self::Context, field: DataField<'v, '_>) {
        T::push_data(ctxt, field).await
    }

    #[inline(always)]
    fn finalize(this: Self::Context) -> Result<'v, Self> {
        T::finalize(this).map(Self)
    }
}

impl<T> std::ops::Deref for Strict<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> std::ops::DerefMut for Strict<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'f, A, T: FromUriParam<Query, A> + FromForm<'f>> FromUriParam<Query, A> for Strict<T> {
    type Target = T::Target;

    #[inline(always)]
    fn from_uri_param(param: A) -> Self::Target {
        T::from_uri_param(param)
    }
}
