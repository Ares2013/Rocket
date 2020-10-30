use indexmap::{IndexMap, IndexSet};
use serde::Serialize;

use crate::http::RawStr;
use crate::form::prelude::*;

#[derive(Debug, Default, Serialize)]
pub struct Context<'v> {
    errors: IndexMap<NameViewCow<'v>, Errors<'v>>,
    values: IndexMap<&'v Name, Vec<&'v RawStr>>,
    data_values: IndexSet<&'v Name>,
    other_errors: Errors<'v>,
}

impl<'v> Context<'v> {
    fn add_errors(&mut self, errors: Errors<'v>) {
        for e in errors {
            if let Some(ref name) = e.name {
                if let Some(errors) = self.errors.get_mut(name) {
                    errors.push(e);
                } else {
                    self.errors.insert(name.clone(), e.into());
                }
            } else {
                self.other_errors.push(e);
            }
        }
    }

    pub fn value<N: AsRef<Name>>(&self, name: N) -> Option<&'v RawStr> {
        self.values.get(name.as_ref())?.get(0).cloned()
    }

    pub fn values<'a, N>(&'a self, name: N) -> impl Iterator<Item = &'v RawStr> + 'a
        where N: AsRef<Name>
    {
        self.values
            .get(name.as_ref())
            .map(|e| e.iter().cloned())
            .into_iter()
            .flatten()
    }

    pub fn has_error<N: AsRef<Name>>(&self, name: &N) -> bool {
        self.errors(name).next().is_some()
    }

    pub fn errors<'a, N>(&'a self, name: &'a N) -> impl Iterator<Item = &Error<'v>>
        where N: AsRef<Name>
    {
        let name = name.as_ref();
        name.prefixes()
            .filter_map(move |name| self.errors.get(name))
            .map(|e| e.iter())
            .flatten()
    }

    pub fn all_errors(&self) -> impl Iterator<Item = &Error<'v>> {
        self.errors.values()
            .map(|e| e.iter())
            .flatten()
            .chain(self.other_errors.iter())
    }
}

// use crate::request::Request;
// use crate::data::{Data, FromTransformedData, Outcome, Transform};

// #[crate::async_trait]
// impl<'r, T: FromForm<'r> + 'r> FromTransformedData<'r> for ContextForm<'r, T> {
//     type Error = Context<'r>;
//     type Owned = <Form<Self> as FromTransformedData<'r>>::Owned;
//     type Borrowed = <Form<Self> as FromTransformedData<'r>>::Borrowed;
//
//     async fn transform(
//         req: &'r Request<'_>,
//         data: Data
//     ) -> Outcome<Transform<Self::Owned>, Self::Error> {
//         <Form<Self> as FromTransformedData<'_>>::transform(req, data).await
//             .map_failure(|(s, e)| (s, Context::from(e)))
//     }
//
//     async fn from_data(
//         req: &'r Request<'_>,
//         transform: Transform<Self::Owned, &'r mut Self::Borrowed>
//     ) -> Outcome<ContextForm<'r, T>, Context<'r>> {
//         <Form<Self> as FromTransformedData<'_>>::from_data(req, transform).await
//     }
// }

// struct ContextForm<'v, T> {
//     pub inner: Option<T>,
//     pub context: Context<'v>
// }
//
// // What we want is for `Context` to contain all of the context up to the point
// // that an error occured. We also don't want to rewrite or duplicate
// // `parse_form`. The issue is that when an external error occurs, we discard the
// // form value itself, hence discarding the context.
//
// #[crate::async_trait]
// impl<'v, T: FromForm<'v> + 'v> FromForm<'v> for ContextForm<'v, T> {
//     type Context = (<T as FromForm<'v>>::Context, Context<'v>);
//
//     fn init(opts: Options) -> Self::Context {
//         (T::init(opts), Context::default())
//     }
//
//     fn push_value((ref mut val_ctxt, ctxt): &mut Self::Context, field: ValueField<'v>) {
//         ctxt.values.entry(field.name.source()).or_default().push(field.value);
//         T::push_value(val_ctxt, field);
//     }
//
//     async fn push_data(
//         (ref mut val_ctxt, ctxt): &mut Self::Context,
//         field: DataField<'v, '_>
//     ) {
//         ctxt.data_values.insert(field.name.source());
//         T::push_data(val_ctxt, field).await;
//     }
//
//     fn finalize((val_ctxt, mut context): Self::Context) -> Result<'v, Self> {
//         let inner = match T::finalize(val_ctxt) {
//             Ok(value) => Some(value),
//             Err(errors) => {
//                 context.add_errors(errors);
//                 None
//             }
//         };
//
//         Ok(ContextForm { inner, context })
//     }
//
//     // fn default() -> Option<Self> {
//     //     Some(ContextForm {
//     //         inner: T::default(),
//     //         context: Context::default()
//     //     })
//     // }
// }
//
// impl<'f> From<Errors<'f>> for Context<'f> {
//     fn from(errors: Errors<'f>) -> Self {
//         let mut context = Context::default();
//         context.add_errors(errors);
//         context
//     }
// }
