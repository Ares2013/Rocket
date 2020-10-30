// use crate::form::{Form, StrictForm, FromForm, Errors};
//
// /// Marker trait for [`FromForm`] types parsable from query strings.
// ///
// /// # Query Guards
// ///
// /// A query guard operates on multiple items of a request's query string. It
// /// validates and optionally converts a query string into another value.
// /// Validation and parsing/conversion is implemented through `FromQuery`. In
// /// other words, every type that implements `FromQuery` is a query guard.
// ///
// /// Query guards are used as the target of trailing query parameters, which
// /// syntactically take the form `<param..>` after a `?` in a route's path. For
// /// example, the parameter `user` is a trailing query parameter in the following
// /// route:
// ///
// /// ```rust
// /// # #[macro_use] extern crate rocket;
// /// use rocket::request::Form;
// ///
// /// #[derive(FromForm)]
// /// struct User {
// ///     name: String,
// ///     account: usize,
// /// }
// ///
// /// #[get("/item?<id>&<user..>")]
// /// fn item(id: usize, user: Form<User>) { /* ... */ }
// /// # fn main() { }
// /// ```
// ///
// /// The `FromQuery` implementation of `Form<User>` will be passed in a [`Query`]
// /// that iterates over all of the query items that don't have the key `id`
// /// (because of the `<id>` dynamic query parameter). For posterity, note that
// /// the `value` of an `id=value` item in a query string will be parsed as a
// /// `usize` and passed in to `item` as `id`.
// ///
// /// # Forwarding
// ///
// /// If the conversion fails, signaled by returning an `Err` from a `FromQuery`
// /// implementation, the incoming request will be forwarded to the next matching
// /// route, if any. For instance, in the `item` route above, if a query string is
// /// missing either a `name` or `account` key/value pair, or there is a query
// /// item with a key that is not `id`, `name`, or `account`, the request will be
// /// forwarded. Note that this strictness is imposed by the [`Form`] type. As an
// /// example, using the [`LenientForm`] type instead would allow extra form items
// /// to be ignored without forwarding. Alternatively, _not_ having a trailing
// /// parameter at all would result in the same.
// ///
// /// # Provided Implementations
// ///
// /// Rocket implements `FromQuery` for several standard types. Their behavior is
// /// documented here.
// ///
// ///   * **Form&lt;T>** _where_ **T: FromForm**
// ///
// ///     Parses the query as a strict form, where each key is mapped to a field
// ///     in `T`. See [`Form`] for more information.
// ///
// ///   * **LenientForm&lt;T>** _where_ **T: FromForm**
// ///
// ///     Parses the query as a lenient form, where each key is mapped to a field
// ///     in `T`. See [`LenientForm`] for more information.
// ///
// ///   * **Option&lt;T>** _where_ **T: FromQuery**
// ///
// ///     _This implementation always returns successfully._
// ///
// ///     The query is parsed by `T`'s `FromQuery` implementation. If the parse
// ///     succeeds, a `Some(parsed_value)` is returned. Otherwise, a `None` is
// ///     returned.
// ///
// ///   * **Result&lt;T, T::Error>** _where_ **T: FromQuery**
// ///
// ///     _This implementation always returns successfully._
// ///
// ///     The path segment is parsed by `T`'s `FromQuery` implementation. The
// ///     returned `Result` value is returned.
// ///
// /// # Example
// ///
// /// Explicitly implementing `FromQuery` should be rare. For most use-cases, a
// /// query guard of `Form<T>` or `LenientForm<T>`, coupled with deriving
// /// `FromForm` (as in the previous example) will suffice. For special cases
// /// however, an implementation of `FromQuery` may be warranted.
// ///
// /// Consider a contrived scheme where we expect to receive one query key, `key`,
// /// three times and wish to take the middle value. For instance, consider the
// /// query:
// ///
// /// ```text
// /// key=first_value&key=second_value&key=third_value
// /// ```
// ///
// /// We wish to extract `second_value` from this query into a `Contrived` struct.
// /// Because `Form` and `LenientForm` will take the _last_ value (`third_value`
// /// here) and don't check that there are exactly three keys named `key`, we
// /// cannot make use of them and must implement `FromQuery` manually. Such an
// /// implementation might look like:
// ///
// /// ```rust
// /// use rocket::http::RawStr;
// /// use rocket::request::{Query, FromQuery};
// ///
// /// /// Our custom query guard.
// /// struct Contrived<'q>(&'q RawStr);
// ///
// /// impl<'q> FromQuery<'q> for Contrived<'q> {
// ///     /// The number of `key`s we actually saw.
// ///     type Error = usize;
// ///
// ///     fn from_query(query: Query<'q>) -> Result<Self, Self::Error> {
// ///         let mut key_items = query.filter(|i| i.key == "key");
// ///
// ///         // This is cloning an iterator, which is cheap.
// ///         let count = key_items.clone().count();
// ///         if count != 3 {
// ///             return Err(count);
// ///         }
// ///
// ///         // The `ok_or` gets us a `Result`. We will never see `Err(0)`.
// ///         key_items.map(|i| Contrived(i.value)).nth(1).ok_or(0)
// ///     }
// /// }
// /// ```
// // pub trait FromQuery<'q>: FromForm<'q> {}
// //
// // impl<'q, T: FromQuery<'q> + FromForm<'q>> FromQuery<'q> for Option<T> { }
// //
// // impl<'q, T: FromQuery<'q> + FromForm<'q>> FromQuery<'q> for Result<T, Errors<'q>> { }
// //
// // impl<'q, T: FromForm<'q>> FromQuery<'q> for Form<T> { }
// //
// // impl<'q, T: FromForm<'q>> FromQuery<'q> for StrictForm<T> { }
