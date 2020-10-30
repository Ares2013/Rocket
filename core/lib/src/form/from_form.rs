use std::borrow::Cow;
use std::collections::{HashMap, BTreeMap};
use std::hash::Hash;

use either::Either;
use indexmap::IndexMap;

use crate::form::prelude::*;
use crate::http::uncased::AsUncased;

#[crate::async_trait]
pub trait FromForm<'v>: Send + Sized {
    type Context: Send;

    fn init(opts: Options) -> Self::Context;

    fn push_value(ctxt: &mut Self::Context, field: ValueField<'v>);

    async fn push_data(ctxt: &mut Self::Context, field: DataField<'v, '_>);

    fn finalize(this: Self::Context) -> Result<'v, Self>;

    fn default() -> Option<Self> {
        Self::finalize(Self::init(Options::Lenient)).ok()
    }
}

#[doc(hidden)]
pub struct VecContext<'v, T: FromForm<'v>> {
    opts: Options,
    last_key: Option<&'v Key>,
    current: Option<T::Context>,
    errors: Errors<'v>,
    items: Vec<T>
}

impl<'v, T: FromForm<'v>> VecContext<'v, T> {
    fn shift(&mut self) {
        if let Some(current) = self.current.take() {
            match T::finalize(current) {
                Ok(v) => self.items.push(v),
                Err(e) => self.errors.extend(e)
            }
        }
    }

    fn context(&mut self, name: &NameView<'v>) -> &mut T::Context {
        // eprintln!("key: {:?}, last: {:?}", name.key(), self.last_key);
        let this_key = name.key();
        let keys_match = match (self.last_key, this_key) {
            (Some(k1), Some(k2)) if k1 == k2 => true,
            _ => false
        };

        if !keys_match {
            self.shift();
            self.current = Some(T::init(self.opts));
        }

        self.last_key = name.key();
        self.current.as_mut().expect("must have current if last == index")
    }
}

#[crate::async_trait]
impl<'v, T: FromForm<'v> + 'v> FromForm<'v> for Vec<T> {
    type Context = VecContext<'v, T>;

    fn init(opts: Options) -> Self::Context {
        VecContext {
            opts,
            last_key: None,
            current: None,
            items: vec![],
            errors: Errors::new(),
        }
    }

    fn push_value(this: &mut Self::Context, field: ValueField<'v>) {
        T::push_value(this.context(&field.name), field.shift());
    }

    async fn push_data(ctxt: &mut Self::Context, field: DataField<'v, '_>) {
        T::push_data(ctxt.context(&field.name), field.shift()).await
    }

    fn finalize(mut this: Self::Context) -> Result<'v, Self> {
        this.shift();
        match this.errors.is_empty() {
            true => Ok(this.items),
            false => Err(this.errors)?,
        }
    }
}

#[doc(hidden)]
pub struct MapContext<'v, K, V> where K: FromForm<'v>, V: FromForm<'v> {
    opts: Options,
    /// Maps from the string key to the index in `map`.
    key_map: IndexMap<&'v str, (usize, NameView<'v>)>,
    keys: Vec<K::Context>,
    values: Vec<V::Context>,
    errors: Errors<'v>,
}

impl<'v, K, V> MapContext<'v, K, V>
    where K: FromForm<'v>, V: FromForm<'v>
{
    fn new(opts: Options) -> Self {
        MapContext {
            opts,
            key_map: IndexMap::new(),
            keys: vec![],
            values: vec![],
            errors: Errors::new(),
        }
    }

    fn ctxt(&mut self, key: &'v str, name: NameView<'v>) -> (&mut K::Context, &mut V::Context) {
        match self.key_map.get(key) {
            Some(&(i, _)) => (&mut self.keys[i], &mut self.values[i]),
            None => {
                debug_assert_eq!(self.keys.len(), self.values.len());
                let map_index = self.keys.len();
                self.keys.push(K::init(self.opts));
                self.values.push(V::init(self.opts));
                self.key_map.insert(key, (map_index, name));
                (self.keys.last_mut().unwrap(), self.values.last_mut().unwrap())
            }
        }
    }

    fn push(
        &mut self,
        name: NameView<'v>
    ) -> Option<Either<&mut K::Context, &mut V::Context>> {
        let index_pair = name.key()
            .map(|k| k.indices())
            .map(|mut i| (i.next(), i.next()))
            .unwrap_or_default();

        match index_pair {
            (Some(key), None) => {
                let is_new_key = !self.key_map.contains_key(key);
                let (key_ctxt, val_ctxt) = self.ctxt(key, name);
                if is_new_key {
                    K::push_value(key_ctxt, ValueField::from_value(key));
                }

                return Some(Either::Right(val_ctxt));
            },
            (Some(kind), Some(key)) => {
                if kind.as_uncased().starts_with("k") {
                    return Some(Either::Left(self.ctxt(key, name).0));
                } else if kind.as_uncased().starts_with("v") {
                    return Some(Either::Right(self.ctxt(key, name).1));
                } else {
                    let error = Error::from(&[Cow::Borrowed("k"), Cow::Borrowed("v")])
                        .with_entity(Entity::Index(0))
                        .with_name(name);

                    self.errors.push(error);
                }
            }
            _ => {
                let error = Error::from(ErrorKind::Missing)
                    .with_entity(Entity::Indices)
                    .with_name(name);

                self.errors.push(error);
            }
        };

        None
    }

    fn push_value(&mut self, field: ValueField<'v>) {
        match self.push(field.name) {
            Some(Either::Left(ctxt)) => K::push_value(ctxt, field.shift()),
            Some(Either::Right(ctxt)) => V::push_value(ctxt, field.shift()),
            _ => {}
        }
    }

    async fn push_data(&mut self, field: DataField<'v, '_>) {
        match self.push(field.name) {
            Some(Either::Left(ctxt)) => K::push_data(ctxt, field.shift()).await,
            Some(Either::Right(ctxt)) => V::push_data(ctxt, field.shift()).await,
            _ => {}
        }
    }

    fn finalize<T: std::iter::FromIterator<(K, V)>>(self) -> Result<'v, T> {
        let (keys, values, key_map) = (self.keys, self.values, self.key_map);
        let errors = std::cell::RefCell::new(self.errors);

        let keys = keys.into_iter()
            .zip(key_map.values().map(|(_, name)| name))
            .filter_map(|(ctxt, name)| match K::finalize(ctxt) {
                Ok(value) => Some(value),
                Err(e) => { errors.borrow_mut().extend(e.with_name(*name)); None }
            });

        let values = values.into_iter()
            .zip(key_map.values().map(|(_, name)| name))
            .filter_map(|(ctxt, name)| match V::finalize(ctxt) {
                Ok(value) => Some(value),
                Err(e) => { errors.borrow_mut().extend(e.with_name(*name)); None }
            });

        let map: T = keys.zip(values).collect();
        let no_errors = errors.borrow().is_empty();
        match no_errors {
            true => Ok(map),
            false => Err(errors.into_inner())
        }
    }
}

#[crate::async_trait]
impl<'v, K, V> FromForm<'v> for HashMap<K, V>
    where K: FromForm<'v> + Eq + Hash, V: FromForm<'v>
{
    type Context = MapContext<'v, K, V>;

    fn init(opts: Options) -> Self::Context {
        MapContext::new(opts)
    }

    fn push_value(ctxt: &mut Self::Context, field: ValueField<'v>) {
        ctxt.push_value(field);
    }

    async fn push_data(ctxt: &mut Self::Context, field: DataField<'v, '_>) {
        ctxt.push_data(field).await;
    }

    fn finalize(this: Self::Context) -> Result<'v, Self> {
        this.finalize()
    }
}

#[crate::async_trait]
impl<'v, K, V> FromForm<'v> for BTreeMap<K, V>
    where K: FromForm<'v> + Ord, V: FromForm<'v>
{
    type Context = MapContext<'v, K, V>;

    fn init(opts: Options) -> Self::Context {
        MapContext::new(opts)
    }

    fn push_value(ctxt: &mut Self::Context, field: ValueField<'v>) {
        ctxt.push_value(field);
    }

    async fn push_data(ctxt: &mut Self::Context, field: DataField<'v, '_>) {
        ctxt.push_data(field).await;
    }

    fn finalize(this: Self::Context) -> Result<'v, Self> {
        this.finalize()
    }
}

#[doc(hidden)]
pub struct PairContext<'v, A: FromForm<'v>, B: FromForm<'v>> {
    left: A::Context,
    right: B::Context,
    errors: Errors<'v>,
}

#[crate::async_trait]
impl<'v, A: FromForm<'v>, B: FromForm<'v>> FromForm<'v> for (A, B) {
    type Context = PairContext<'v, A, B>;

    fn init(opts: Options) -> Self::Context {
        PairContext {
            left: A::init(opts),
            right: B::init(opts),
            errors: Errors::new()
        }
    }

    // a[b].c
    fn push_value(c: &mut Self::Context, field: ValueField<'v>) {
        match field.name.key_lossy().as_str() {
            "0" => A::push_value(&mut c.left, field.shift()),
            "1" => B::push_value(&mut c.right, field.shift()),
            key => {
                A::push_value(&mut c.left, ValueField::from_value(key));
                B::push_value(&mut c.right, field.shift());
            }
        }
    }

    async fn push_data(c: &mut Self::Context, field: DataField<'v, '_>) {
        match field.name.key_lossy().as_str() {
            "0" => A::push_data(&mut c.left, field.shift()).await,
            "1" => B::push_data(&mut c.right, field.shift()).await,
            key => {
                A::push_value(&mut c.left, ValueField::from_value(key));
                B::push_data(&mut c.right, field.shift()).await
            }
        }
    }

    fn finalize(mut this: Self::Context) -> Result<'v, Self> {
        match (A::finalize(this.left), B::finalize(this.right)) {
            (Ok(key), Ok(val)) if this.errors.is_empty() => Ok((key, val)),
            (Ok(_), Ok(_)) => Err(this.errors)?,
            (left, right) => {
                if let Err(e) = left { this.errors.extend(e); }
                if let Err(e) = right { this.errors.extend(e); }
                Err(this.errors)?
            }
        }
    }
}

#[crate::async_trait]
impl<'v, T: FromForm<'v>> FromForm<'v> for Option<T> {
    type Context = <T as FromForm<'v>>::Context;

    fn init(opts: Options) -> Self::Context {
        T::init(opts)
    }

    fn push_value(ctxt: &mut Self::Context, field: ValueField<'v>) {
        T::push_value(ctxt, field)
    }

    async fn push_data(ctxt: &mut Self::Context, field: DataField<'v, '_>) {
        T::push_data(ctxt, field).await
    }

    fn finalize(this: Self::Context) -> Result<'v, Self> {
        match T::finalize(this) {
            Ok(v) => Ok(Some(v)),
            Err(_) => Ok(None)
        }
    }
}

#[crate::async_trait]
impl<'v, T: FromForm<'v>> FromForm<'v> for Result<'v, T> {
    type Context = <T as FromForm<'v>>::Context;

    fn init(opts: Options) -> Self::Context {
        T::init(opts)
    }

    fn push_value(ctxt: &mut Self::Context, field: ValueField<'v>) {
        T::push_value(ctxt, field)
    }

    async fn push_data(ctxt: &mut Self::Context, field: DataField<'v, '_>) {
        T::push_data(ctxt, field).await
    }

    fn finalize(this: Self::Context) -> Result<'v, Self> {
        match T::finalize(this) {
            Ok(v) => Ok(Ok(v)),
            Err(e) => Ok(Err(e))
        }
    }
}
