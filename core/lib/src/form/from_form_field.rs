use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6, SocketAddr};
use std::num::{
    NonZeroIsize, NonZeroI8, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI128,
    NonZeroUsize, NonZeroU8, NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU128,
};

use crate::data::Capped;
use crate::http::uncased::AsUncased;
use crate::form::prelude::*;

use time::{Date, PrimitiveDateTime};

// Ideally, for type safety reasons, especially when dealing with query values
// (which we'd like to have use `FromFormValue` instead of `FromFormField`) this
// would be two traits: `FromFormValue` and `FromFormField`. However, given
// blanket implementations for `FromForm` for implementors of said traits, this
// would result in duplicate implementations of `FromForm` for types that
// implement both traits. We need specialization to resolve this concern. Thus,
// for now, we keep this as one trait.
#[crate::async_trait]
pub trait FromFormField<'v>: Send + Sized {
    fn default() -> Option<Self> { None }

    fn from_value(field: ValueField<'v>) -> Result<'v, Self> {
        Err(field.unexpected())?
    }

    async fn from_data(field: DataField<'v, '_>) -> Result<'v, Self> {
        Err(field.unexpected())?
    }
}

#[doc(hidden)]
pub struct FromFieldContext<'v, T: FromFormField<'v>> {
    field_name: Option<NameView<'v>>,
    field_value: Option<&'v str>,
    opts: Options,
    value: Option<Result<'v, T>>,
    pushes: usize
}

impl<'v, T: FromFormField<'v>> FromFieldContext<'v, T> {
    fn can_push(&mut self) -> bool {
        self.pushes += 1;
        self.value.is_none()
    }

    fn push(&mut self, name: NameView<'v>, result: Result<'v, T>) {
        let is_unexpected = |e: &Errors<'_>| e.last().map_or(false, |e| {
            if let ErrorKind::Unexpected = e.kind { true } else { false }
        });

        self.field_name = Some(name);
        match result {
            Err(e) if !self.opts.strict && is_unexpected(&e) => { /* ok */ },
            result => self.value = Some(result),
        }
    }
}

#[crate::async_trait]
impl<'v, T: FromFormField<'v>> FromForm<'v> for T {
    type Context = FromFieldContext<'v, T>;

    fn init(opts: Options) -> Self::Context {
        FromFieldContext {
            opts,
            field_name: None,
            field_value: None,
            value: None,
            pushes: 0,
        }
    }

    fn push_value(ctxt: &mut Self::Context, field: ValueField<'v>) {
        if ctxt.can_push() {
            ctxt.field_value = Some(field.value);
            ctxt.push(field.name, Self::from_value(field))
        }
    }

    async fn push_data(ctxt: &mut FromFieldContext<'v, T>, field: DataField<'v, '_>) {
        if ctxt.can_push() {
            ctxt.push(field.name, Self::from_data(field).await);
        }
    }

    fn finalize(ctxt: Self::Context) -> Result<'v, Self> {
        let mut errors = match ctxt.value {
            Some(Ok(val)) if !ctxt.opts.strict || ctxt.pushes <= 1 => return Ok(val),
            Some(Err(e)) => e,
            Some(Ok(_)) => Errors::from(ErrorKind::Duplicate),
            None => match <T as FromFormField>::default() {
                Some(default) => return Ok(default),
                None => Errors::from(ErrorKind::Missing)
            }
        };

        if let Some(name) = ctxt.field_name {
            errors.set_name(name);
        }

        if let Some(value) = ctxt.field_value {
            errors.set_value(value);
        }

        Err(errors.with_context(std::any::type_name::<T>()))
    }
}

impl<'v> FromFormField<'v> for &'v str {
    fn from_value(field: ValueField<'v>) -> Result<'v, Self> {
        Ok(field.value)
    }
}

#[crate::async_trait]
impl<'v> FromFormField<'v> for Capped<String> {
    fn from_value(field: ValueField<'v>) -> Result<'v, Self> {
        Ok(Capped::from(field.value.to_string()))
    }

    async fn from_data(f: DataField<'v, '_>) -> Result<'v, Self> {
        use crate::data::{Capped, Outcome, FromData};

        match <Capped<String> as FromData>::from_data(f.request, f.data).await {
            Outcome::Success(p) => Ok(p),
            Outcome::Failure((_, e)) => Err(e)?,
            Outcome::Forward(..) => {
                Err(Error::from(ErrorKind::Unexpected).with_entity(Entity::DataField))?
            }
        }
    }
}

impl<'v> FromFormField<'v> for ValueField<'v> {
    fn from_value(field: ValueField<'v>) -> Result<'v, Self> {
        Ok(field)
    }
}

#[crate::async_trait]
impl<'v> FromFormField<'v> for crate::data::Data {
    async fn from_data(field: DataField<'v, '_>) -> Result<'v, Self> {
        Ok(field.data)
    }
}

impl_strict_from_form_field_from_capped!(String);

impl<'v> FromFormField<'v> for bool {
    fn default() -> Option<Self> { Some(false) }

    fn from_value(field: ValueField<'v>) -> Result<'v, Self> {
        match field.value.as_uncased() {
            v if v == "on" || v == "yes" || v == "true" => Ok(true),
            v if v == "off" || v == "no" || v == "false" => Ok(false),
            // force a `ParseBoolError`
            _ => Ok("".parse()?),
        }
    }
}

macro_rules! impl_with_parse {
    ($($T:ident),+ $(,)?) => ($(
        impl<'v> FromFormField<'v> for $T {
            #[inline(always)]
            fn from_value(field: ValueField<'v>) -> Result<'v, Self> {
                Ok(field.value.parse()?)
            }
        }
    )+)
}

impl_with_parse!(
    f32, f64,
    isize, i8, i16, i32, i64, i128,
    usize, u8, u16, u32, u64, u128,
    NonZeroIsize, NonZeroI8, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI128,
    NonZeroUsize, NonZeroU8, NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU128,
    Ipv4Addr, IpAddr, Ipv6Addr, SocketAddrV4, SocketAddrV6, SocketAddr
);

impl<'v> FromFormField<'v> for Date {
    fn from_value(field: ValueField<'v>) -> Result<'v, Self> {
        let date = Self::parse(field.value, "%F")
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send>)?;

        Ok(date)
    }
}

// TODO: Doc that we don't support %FT%T.millisecond version.
impl<'v> FromFormField<'v> for PrimitiveDateTime {
    fn from_value(field: ValueField<'v>) -> Result<'v, Self> {
        let dt = Self::parse(field.value, "%FT%R")
            .or_else(|_| Self::parse(field.value, "%FT%T"))
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send>)?;

        Ok(dt)
    }
}
