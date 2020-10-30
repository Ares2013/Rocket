// use std::borrow::{Borrow, BorrowMut};
// use std::convert::Infallible;
//
// use futures::future::BoxFuture;
//
// use crate::outcome::Outcome::*;
// use crate::request::Request;
// use crate::data::{Data, FromData, Outcome};
//
// /// Indicates whether data should be borrowed before calling
// /// [`from_data()`](FromTransformedData::from_data()).
// ///
// /// See the documentation for [`FromTransformedData`] for usage details.
// pub enum Transform<T, B = T> {
//     /// Indicates that data should be or has been transformed into the
//     /// [`FromTransformedData::Owned`] variant.
//     Owned(T),
//
//     /// Indicates that data should be or has been transformed into the
//     /// [`FromTransformedData::Borrowed`] variant.
//     Borrowed(B)
// }
//
// impl<T, B> Transform<T, B> {
//     /// Returns the `Owned` value if `self` is `Owned`.
//     ///
//     /// # Panics
//     ///
//     /// Panics if `self` is `Borrowed`.
//     ///
//     ///
//     /// # Example
//     ///
//     /// ```rust
//     /// use rocket::data::Transform;
//     ///
//     /// let owned: Transform<usize, &[usize]> = Transform::Owned(10);
//     /// assert_eq!(owned.owned(), 10);
//     /// ```
//     #[inline]
//     #[track_caller]
//     pub fn owned(self) -> T {
//         match self {
//             Transform::Owned(val) => val,
//             Transform::Borrowed(_) => panic!("Transform::owned() called on Borrowed"),
//         }
//     }
//
//     /// Returns the `Borrowed` value if `self` is `Borrowed`.
//     ///
//     /// # Panics
//     ///
//     /// Panics if `self` is `Owned`.
//     ///
//     /// ```rust
//     /// use rocket::data::Transform;
//     ///
//     /// let borrowed: Transform<usize, &[usize]> = Transform::Borrowed(&[10]);
//     /// assert_eq!(borrowed.borrowed(), &[10]);
//     /// ```
//     #[inline]
//     #[track_caller]
//     pub fn borrowed(self) -> B {
//         match self {
//             Transform::Borrowed(val) => val,
//             Transform::Owned(_) => panic!("Transform::borrowed() called on Owned"),
//         }
//     }
// }
//
// /// Trait implemented by data guards to derive a value from request body data.
// ///
// /// # Data Guards
// ///
// /// A data guard is a [request guard] that operates on a request's body data.
// /// Data guards validate, parse, and optionally convert request body data.
// /// Validation and parsing/conversion is implemented through
// /// `FromTransformedData`. In other words, every type that implements
// /// `FromTransformedData` is a data guard.
// ///
// /// Data guards are used as the target of the `data` route attribute parameter.
// /// A handler can have at most one data guard. In the example below, `var` is
// /// used as the argument name for the data guard type `DataGuard`. When the
// /// `submit` route matches, Rocket will call the `FromTransformedData`
// /// implementation for the type `T`. The handler will only be called if the
// /// guard returns successfully.
// ///
// /// ```rust
// /// # #[macro_use] extern crate rocket;
// /// # type DataGuard = rocket::data::Data;
// /// #[post("/submit", data = "<var>")]
// /// fn submit(var: DataGuard) { /* ... */ }
// /// # fn main() { }
// /// ```
// ///
// /// For many data guards, implementing [`FromData`] will be simpler and
// /// sufficient; only guards needing to _transform_ data should implement
// /// `FromTransformedData`. All types that implement `FromData` automatically
// /// implement `FromTransformedData`. Thus, when possible, prefer to implement
// /// [`FromData`] instead of `FromTransformedData`.
// ///
// /// # Transforming
// ///
// /// Data guards implementing `FromTransformedData` can optionally _transform_
// /// incoming data before processing it via an implementation of the
// /// [`FromTransformedData::transform()`] method. This is useful when a data
// /// guard requires or could benefit from a reference to body data as opposed to
// /// an owned version. If a data guard has no need to operate on a reference to
// /// body data, [`FromData`] should be implemented instead; it is simpler to
// /// implement and less error prone. All types that implement `FromData`
// /// automatically implement `FromTransformedData`.
// ///
// /// When exercising a data guard, Rocket first calls the guard's
// /// [`FromTransformedData::transform()`] method and awaits on the returned
// /// future. The resulting value is stored on a request-local the stack. If
// /// `transform` returned a [`Transform::Owned`], Rocket moves the data back to
// /// the data guard in the subsequent `from_data` call as a `Transform::Owned`.
// /// If instead `transform` returned a [`Transform::Borrowed`] variant, Rocket
// /// calls `borrow_mut()` on the owned value, producing a mutable borrow of the
// /// associated type [`FromTransformedData::Borrowed`] and passing it `from_data`
// /// as a `Transform::Borrowed`.
// ///
// /// ## Async Trait
// ///
// /// [`FromTransformedData`] is an _async_ trait. Implementations must be
// /// decorated with an attribute of `#[rocket::async_trait]`:
// ///
// /// ```rust
// /// use rocket::request::Request;
// /// use rocket::data::{self, Data, FromTransformedData, Transform};
// ///
// /// # struct MyType;
// /// # struct MyError;
// /// #[rocket::async_trait]
// /// impl<'r> FromTransformedData<'r> for MyType {
// ///     type Error = MyError;
// ///     type Owned = MyType;
// ///     type Borrowed = MyType;
// ///
// ///     #[inline(always)]
// ///     async fn transform(
// ///         req: &'r Request<'_>,
// ///         data: Data
// ///     ) -> data::Outcome<Transform<Self::Owned>, Self::Error> {
// ///         /* ... */
// ///         # unimplemented!()
// ///     }
// ///
// ///     #[inline(always)]
// ///     async fn from_data(
// ///         request: &'r Request<'_>,
// ///         transform: Transform<Self::Owned, &'r mut Self::Borrowed>
// ///     ) -> data::Outcome<Self, Self::Error> {
// ///         /* ... */
// ///         # unimplemented!()
// ///     }
// /// }
// /// ```
// ///
// /// ## Example
// ///
// /// Consider a data guard type that wishes to hold a slice to two different
// /// parts of the incoming data:
// ///
// /// ```rust
// /// struct Name<'a> {
// ///     first: &'a str,
// ///     last: &'a str
// /// }
// /// ```
// ///
// /// Without the ability to transform into a borrow, implementing such a data
// /// guard would be impossible. With transformation, however, we can instruct
// /// Rocket to produce a borrow to a `Data` that has been transformed into a
// /// borrowed `String` (an `&str`).
// ///
// /// ```rust
// /// # #[macro_use] extern crate rocket;
// /// # #[derive(Debug)]
// /// # struct Name<'a> { first: &'a str, last: &'a str, }
// /// use std::io;
// ///
// /// use rocket::request::Request;
// /// use rocket::outcome::Outcome::*;
// /// use rocket::data::{self, Data, FromTransformedData, Transform, ToByteUnit};
// /// use rocket::http::Status;
// ///
// /// enum NameError {
// ///     Io(io::Error),
// ///     TooLarge,
// ///     Parse
// /// }
// ///
// /// #[rocket::async_trait]
// /// impl<'r> FromTransformedData<'r> for Name<'r> {
// ///     type Error = NameError;
// ///     type Owned = String;
// ///     type Borrowed = str;
// ///
// ///     async fn transform(
// ///         req: &'r Request<'_>,
// ///         data: Data
// ///     ) -> data::Outcome<Transform<Self::Owned>, Self::Error> {
// ///         // Use a configured limit or fallback to a default.
// ///         let limit = req.limits().get("name").unwrap_or(256.bytes());
// ///
// ///         match data.open(limit).into_string().await {
// ///             // Returning `Borrowed` here means we get `Borrowed` in `from_data`.
// ///             Ok(s) if s.is_complete() => Success(Transform::Borrowed(s.value)),
// ///             Ok(_) => Failure((Status::PayloadTooLarge, NameError::TooLarge)),
// ///             Err(e) => Failure((Status::InternalServerError, NameError::Io(e)))
// ///         }
// ///     }
// ///
// ///     async fn from_data(
// ///         request: &'r Request<'_>,
// ///         transform: Transform<Self::Owned, &'r mut Self::Borrowed>
// ///     ) -> data::Outcome<Self, Self::Error> {
// ///         // Retrieve a borrow to the now transformed `String` (an &str).
// ///         // This is only correct because we know we _always_ return a
// ///         // `Borrowed` from `transform` above.
// ///         let string = transform.borrowed();
// ///
// ///         // Perform a crude, inefficient parse.
// ///         let splits: Vec<&str> = string.split(" ").collect();
// ///         if splits.len() != 2 || splits.iter().any(|s| s.is_empty()) {
// ///             return Failure((Status::UnprocessableEntity, NameError::Parse));
// ///         }
// ///
// ///         // Return successfully.
// ///         Success(Name { first: splits[0], last: splits[1] })
// ///     }
// /// }
// /// #
// /// # #[post("/person", data = "<person>")]
// /// # fn person(person: Name) {  }
// /// # #[post("/person", data = "<person>")]
// /// # fn person2(person: Result<Name, NameError>) {  }
// /// # fn main() {  }
// /// ```
// ///
// /// # Outcomes
// ///
// /// The returned [`Outcome`] of `from_data` determines how the incoming request
// /// will be processed.
// ///
// /// * **Success**(S)
// ///
// ///   If the `Outcome` is [`Success`], then the `Success` value will be used as
// ///   the value for the data parameter.  As long as all other parsed types
// ///   succeed, the request will be handled by the requesting handler.
// ///
// /// * **Failure**(Status, E)
// ///
// ///   If the `Outcome` is [`Failure`], the request will fail with the given
// ///   status code and error. The designated error [`Catcher`](crate::Catcher)
// ///   will be used to respond to the request. Note that users can request types
// ///   of `Result<S, E>` and `Option<S>` to catch `Failure`s and retrieve the
// ///   error value.
// ///
// /// * **Forward**(Data)
// ///
// ///   If the `Outcome` is [`Forward`], the request will be forwarded to the next
// ///   matching request. This requires that no data has been read from the `Data`
// ///   parameter. Note that users can request an `Option<S>` to catch `Forward`s.
// ///
// /// # Provided Implementations
// ///
// /// Rocket implements `FromTransformedData` for several built-in types. Their
// /// behavior is documented here.
// ///
// ///   * **Data**
// ///
// ///     The identity implementation; simply returns [`Data`] directly.
// ///
// ///     _This implementation always returns successfully._
// ///
// ///   * **Option&lt;T>** _where_ **T: FromTransformedData**
// ///
// ///     The type `T` is derived from the incoming data using `T`'s
// ///     `FromTransformedData` implementation. If the derivation is a `Success`,
// ///     the derived value is returned in `Some`. Otherwise, a `None` is
// ///     returned.
// ///
// ///     _This implementation always returns successfully._
// ///
// ///   * **Result&lt;T, T::Error>** _where_ **T: FromTransformedData**
// ///
// ///     The type `T` is derived from the incoming data using `T`'s
// ///     `FromTransformedData` implementation. If derivation is a `Success`, the
// ///     value is returned in `Ok`. If the derivation is a `Failure`, the error
// ///     value is returned in `Err`. If the derivation is a `Forward`, the
// ///     request is forwarded.
// ///
// ///   * **String** and **Capped&lt;String>**
// ///
// ///     Reads the entire request body into a `String` limited by the `string`
// ///     limit. If reading fails, returns a `Failure` with the corresponding
// ///     `io::Error`.
// ///
// ///   * **Vec&lt;u8>** and **Capped&ltVec<u8>>**
// ///
// ///     Reads the entire request body into a `Vec<u8>` limited by the `bytes`
// ///     limit. If reading fails, returns a `Failure` with the corresponding
// ///     `io::Error`.
// ///
// /// # Simplified `FromTransformedData`
// ///
// /// For an example of a type that wouldn't require transformation, see the
// /// [`FromData`] documentation.
// #[crate::async_trait]
// pub trait FromTransformedData<'r>: Sized {
//     /// The associated error to be returned when the guard fails.
//     type Error: Send;
//
//     /// The owned type returned from [`FromTransformedData::transform()`].
//     ///
//     /// The trait bound ensures that it is is possible to borrow an
//     /// `&mut Self::Borrowed` from a value of this type.
//     type Owned: BorrowMut<Self::Borrowed>;
//
//     /// The _borrowed_ type consumed by [`FromTransformedData::from_data()`] when
//     /// [`FromTransformedData::transform()`] returns a [`Transform::Borrowed`].
//     ///
//     /// If [`FromTransformedData::from_data()`] returns a [`Transform::Owned`], this
//     /// associated type should be set to `Self::Owned`.
//     type Borrowed: ?Sized;
//
//     /// Asynchronously transforms `data` into a value of type `Self::Owned`.
//     ///
//     /// If the returned future resolves to `Transform::Owned(Self::Owned)`, then
//     /// `from_data` should subsequently be called with a `data` value of
//     /// `Transform::Owned(Self::Owned)`. If the future resolves to
//     /// `Transform::Borrowed(Self::Owned)`, `from_data` should subsequently be
//     /// called with a `data` value of `Transform::Borrowed(&Self::Borrowed)`. In
//     /// other words, the variant of `Transform` returned from this method is
//     /// used to determine which variant of `Transform` should be passed to the
//     /// `from_data` method. Rocket _always_ makes the subsequent call correctly.
//     ///
//     /// It is very unlikely that a correct implementation of this method is
//     /// capable of returning either of an `Owned` or `Borrowed` variant.
//     /// Instead, this method should return exactly _one_ of these variants.
//     ///
//     /// If transformation succeeds, an outcome of `Success` is returned.
//     /// If the data is not appropriate given the type of `Self`, `Forward` is
//     /// returned. On failure, `Failure` is returned.
//     async fn transform(
//         request: &'r Request<'_>,
//         data: Data
//     ) -> Outcome<Transform<Self::Owned>, Self::Error>;
//
//     /// Asynchronously validates, parses, and converts the incoming request body
//     /// data into an instance of `Self`.
//     ///
//     /// If validation and parsing succeeds, an outcome of `Success` is returned.
//     /// If the data is not appropriate given the type of `Self`, `Forward` is
//     /// returned. If parsing or validation fails, `Failure` is returned.
//     ///
//     /// When implementing this method, you rarely need to destruct the `outcome`
//     /// parameter. Instead, the first line of the method should be either
//     /// [`transform.owned()`](Transform::owned()) or
//     /// [`tranformed.borrowed()`](Transform::borrowed()).
//     async fn from_data(
//         request: &'r Request<'_>,
//         transform: Transform<Self::Owned, &'r mut Self::Borrowed>
//     ) -> Outcome<Self, Self::Error>;
// }
//
// /// The identity implementation of `FromTransformedData`. Always returns `Success`.
// #[crate::async_trait]
// impl<'r> FromTransformedData<'r> for Data {
//     type Error = Infallible;
//     type Owned = Data;
//     type Borrowed = Data;
//
//     #[inline(always)]
//     async fn transform(_: &'r Request<'_>, d: Data) -> Outcome<Transform<Data>, Infallible> {
//         Success(Transform::Owned(d))
//     }
//
//     #[inline(always)]
//     async fn from_data(
//         _: &'r Request<'_>,
//         transform: Transform<Self, &'r mut Self::Borrowed>
//     ) -> Outcome<Self, Self::Error> {
//         Success(transform.owned())
//     }
// }
//
// #[crate::async_trait]
// impl<'r, T: FromData + 'r> FromTransformedData<'r> for T {
//     type Error = T::Error;
//     type Owned = Data;
//     type Borrowed = Data;
//
//     #[inline(always)]
//     async fn transform(_: &'r Request<'_>, d: Data) -> Outcome<Transform<Data>, T::Error> {
//         Success(Transform::Owned(d))
//     }
//
//     #[inline(always)]
//     async fn from_data(
//         request: &'r Request<'_>,
//         transform: Transform<Self::Owned, &'r mut Self::Borrowed>
//     ) -> Outcome<Self, Self::Error> {
//         T::from_data(request, transform.owned()).await
//     }
// }
//
// pub enum MyResult<T, E> {
//     Ok(T),
//     Err(E)
// }
//
// impl<T, E> Borrow<T> for MyResult<T, E> {
//     fn borrow(&self) -> &T {
//         match self {
//             MyResult::Ok(ref v) => v,
//             MyResult::Err(_) => unreachable!("...")
//         }
//     }
// }
//
// impl<T, E> BorrowMut<T> for MyResult<T, E> {
//     fn borrow_mut(&mut self) -> &mut T {
//         match self {
//             MyResult::Ok(ref mut v) => v,
//             MyResult::Err(_) => unreachable!("...")
//         }
//     }
// }
//
// impl<'r, T: FromTransformedData<'r> + 'r> FromTransformedData<'r> for Result<T, T::Error>
//     where T::Owned: Send
// {
//     type Error = std::convert::Infallible;
//     // type Owned = Result<T::Owned, T::Error>;
//     // type Borrowed = Result<T::Owned, T::Error>;
//     type Owned = MyResult<T::Owned, T::Error>;
//     type Borrowed = T::Owned;
//
//     fn transform<'life0, 'async_trait>(
//         request: &'r Request<'life0>,
//         data: Data,
//     ) -> BoxFuture<'async_trait, Outcome<Transform<Self::Owned>, Self::Error>>
//         where 'r: 'async_trait, 'life0: 'async_trait, Self: 'async_trait,
//     {
//         use MyResult::*;
//         Box::pin(async move {
//             match T::transform(request, data).await {
//                 Success(Transform::Owned(v)) => Success(Transform::Owned(Ok(v))),
//                 Success(Transform::Borrowed(v)) => Success(Transform::Borrowed(Ok(v))),
//                 Failure((_, e)) => Success(Transform::Owned(Err(e))),
//                 Forward(d) => Forward(d)
//             }
//         })
//     }
//
//     fn from_data<'life0, 'async_trait>(
//         req: &'r Request<'life0>,
//         transform: Transform<Self::Owned, &'r mut T::Owned>,
//     ) -> BoxFuture<'async_trait, Outcome<Self, Self::Error>>
//         where 'r: 'async_trait, 'life0: 'async_trait, Self: 'async_trait,
//     {
//         // use MyResult::*;
//         use Transform::*;
//
//         Box::pin(async move {
//             let outcome = match transform {
//                 Owned(MyResult::Ok(val)) => T::from_data(req, Owned(val)),
//                 Borrowed(v) => T::from_data(req, Borrowed(v.borrow_mut())),
//                 Owned(MyResult::Err(e)) => return Success(Err(e)),
//                 // Borrowed(Err(_)) => unreachable!("Borrowed is never Err()"),
//             };
//
//             match outcome.await {
//                 Success(v) => Success(Ok(v)),
//                 Failure((_, e)) => Success(Err(e)),
//                 Forward(d) => Forward(d)
//             }
//         })
//     }
// }
//
// impl<'r, T: FromTransformedData<'r> + 'r> FromTransformedData<'r> for Option<T>
//     where T::Owned: Send
// {
//     type Error = std::convert::Infallible;
//     type Owned = Option<T::Owned>;
//     type Borrowed = Option<T::Owned>;
//
//     fn transform<'life0, 'async_trait>(
//         request: &'r Request<'life0>,
//         data: Data,
//     ) -> BoxFuture<'async_trait, Outcome<Transform<Self::Owned>, Self::Error>>
//         where 'r: 'async_trait, 'life0: 'async_trait, Self: 'async_trait,
//     {
//         Box::pin(async move {
//             match T::transform(request, data).await {
//                 Success(Transform::Owned(v)) => Success(Transform::Owned(Some(v))),
//                 Success(Transform::Borrowed(v)) => Success(Transform::Borrowed(Some(v))),
//                 Failure(_) | Forward(_) => Success(Transform::Owned(None))
//             }
//         })
//     }
//
//     fn from_data<'life0, 'async_trait>(
//         req: &'r Request<'life0>,
//         transform: Transform<Self::Owned, &'r mut Self::Borrowed>,
//     ) -> BoxFuture<'async_trait, Outcome<Self, Self::Error>>
//         where 'r: 'async_trait, 'life0: 'async_trait, Self: 'async_trait,
//     {
//         Box::pin(async move {
//             use Transform::*;
//
//             let outcome = match transform {
//                 Owned(Some(val)) => T::from_data(req, Owned(val)),
//                 Borrowed(Some(ref mut v)) => T::from_data(req, Borrowed(v.borrow_mut())),
//                 Owned(None) | Borrowed(None) => return Success(None),
//             };
//
//             match outcome.await {
//                 Success(v) => Success(Some(v)),
//                 Failure(_) | Forward(_) => Success(None)
//             }
//         })
//     }
// }
