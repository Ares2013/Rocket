use std::path::Path;

use multer::Multipart;

use crate::request::{Request, local_cache};
use crate::data::{Data, FromData, Limits, Outcome};
use crate::form::error::Errors;
use crate::http::uri::{Query, FromUriParam};
use crate::http::RawStr;
use crate::form::prelude::*;

#[derive(Debug)]
pub struct Form<T>(T);

impl Form<()> {
    /// `string` must represent a decoded string.
    pub fn parse_values(string: &str) -> impl Iterator<Item = ValueField<'_>> {
        // WHATWG URL Living Standard 5.1 steps 1, 2, 3.1 - 3.3.
        string.split('&')
            .filter(|s| !s.is_empty())
            .map(ValueField::parse)
    }

    pub fn parse_raw_values(string: &RawStr) -> impl Iterator<Item = (&RawStr, &RawStr)> {
        // WHATWG URL Living Standard 5.1 steps 1, 2, 3.1 - 3.3.
        string.split('&')
            .filter(|s| !s.is_empty())
            .map(|s| ValueField::parse(s.as_str()))
            .map(|f| (f.name.source().as_str().into(), f.value.into()))
    }
}

impl<'v, T: FromForm<'v>> Form<T> {
    pub fn into_inner(self) -> T {
        self.0
    }

    /// `string` must represent a decoded string.
    pub fn parse(string: &'v str) -> Result<'v, T> {
        // WHATWG URL Living Standard 5.1 steps 1, 2, 3.1 - 3.3.
        let mut ctxt = T::init(Options::Lenient);
        Form::parse_values(string).for_each(|f| T::push_value(&mut ctxt, f));
        T::finalize(ctxt)
    }
}

impl<T: for<'a> FromForm<'a> + 'static> Form<T> {
    /// `string` must represent an undecoded string.
    pub fn parse_encoded_raw(string: &RawStr) -> Result<'static, T> {
        use crate::http::ext::IntoOwned;

        let buffer = Buffer::new();
        let mut context = T::init(Options::Lenient);
        for (name, val) in Form::parse_raw_values(string) {
            let (name, val) = (name.url_decode_lossy(), val.url_decode_lossy());
            let name = buffer.push_one(name);
            let val = buffer.push_one(val);
            T::push_value(&mut context, ValueField::from((name, val)))
        }

        T::finalize(context).map_err(|e| e.into_owned())
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

impl<'f, A, T: FromUriParam<Query, A> + FromForm<'f>> FromUriParam<Query, A> for Form<T> {
    type Target = T::Target;

    #[inline(always)]
    fn from_uri_param(param: A) -> Self::Target {
        T::from_uri_param(param)
    }
}

fn sanitize(file_name: &str) -> Option<&str> {
    let file_name = Path::new(file_name)
        .file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.find('.').map(|i| n.split_at(i).0).unwrap_or(n))?;

    if file_name.is_empty()
        || file_name.starts_with(|c| c == '.' || c == '*')
        || file_name.ends_with(|c| c == ':' || c == '>' || c == '<')
        || file_name.contains(|c| c == '/' || c == '\\')
    {
        return None
    }

    Some(file_name)
}

macro_rules! try_or_finalize {
    ($T:ty, $context:expr, $e:expr) => {
        match $e {
            Ok(v) => v,
            Err(e) => match <$T as FromForm<'_>>::finalize($context) {
                Ok(_) => return Err(e.into()),
                Err(mut errors) => {
                    errors.push(e.into());
                    return Err(errors);
                }
            },
        }
    };
}

#[doc(hidden)]
enum Storage<'r> {
    Str(&'r Buffer, &'r RawStr),
    MultiPart(&'r Buffer, Multipart)
}

async fn parse_form<'r, T: FromForm<'r>>(
    request: &'r Request<'_>,
    mut storage: Storage<'r>,
    options: Options
) -> Result<'r, T> {
    let mut context = T::init(options);
    match storage {
        Storage::Str(buffer, form) => {
            use std::borrow::Cow::*;

            for (name, val) in Form::parse_raw_values(form) {
                let name_val = match (name.url_decode_lossy(), val.url_decode_lossy()) {
                    (Borrowed(name), Borrowed(val)) => (name, val),
                    (Borrowed(name), Owned(v)) => (name, buffer.push_one(v)),
                    (Owned(n), Borrowed(val)) => (buffer.push_one(n), val),
                    (Owned(mut n), Owned(v)) => {
                        let len = n.len();
                        n.push_str(&v);
                        buffer.push_split(n, len)
                    }
                };

                T::push_value(&mut context, ValueField::from(name_val))
            }

            T::finalize(context)
        }
        Storage::MultiPart(buffer, ref mut mp) => loop {
            trace_!("fetching next multipart field");

            let field = match try_or_finalize!(T, context, mp.next_field().await) {
                Some(field) => field,
                None => return T::finalize(context)
            };

            trace_!("multipart field: {:?}", field.name());

            // A field with a content-type is data; one without is "value".
            let ct = field.content_type().and_then(|m| m.as_ref().parse().ok());
            if let Some(content_type) = ct {
                let (name, file_name) = match (field.name(), field.file_name()) {
                    (None, None) => ("", None),
                    (None, Some(file_name)) => ("", Some(buffer.push_one(file_name))),
                    (Some(name), None) => (buffer.push_one(name), None),
                    (Some(a), Some(b)) => {
                        let (field_name, file_name) = buffer.push_two(a, b);
                        (field_name, Some(file_name))
                    }
                };

                let file_name = file_name.and_then(sanitize);

                let data = Data::from(field);
                T::push_data(&mut context, DataField {
                    name: NameView::new(name), file_name, content_type, data, request
                }).await;
            } else {
                let (mut buf, len) = match field.name() {
                    Some(s) => (s.to_string(), s.len()),
                    None => (String::new(), 0)
                };

                let text = try_or_finalize!(T, context, field.text().await);
                buf.push_str(&text);

                let name_val = buffer.push_split(buf, len);
                T::push_value(&mut context, ValueField::from(name_val));
            }
        }
    }
}

impl<'r> Storage<'r> {
    async fn string(req: &'r Request<'_>, data: Data) -> Result<'r, Storage<'r>> {
        let limit = req.limits().get("form").unwrap_or(Limits::FORM);
        let string = data.open(limit).into_string().await?;
        if !string.is_complete() {
            Err((None, Some(limit.as_u64())))?
        }

        let string = local_cache!(req, string.into_inner());
        let buffer = local_cache!(req, Buffer::new());
        Ok(Storage::Str(buffer, RawStr::new(string)))
    }

    async fn multipart(req: &'r Request<'_>, data: Data) -> Result<'r, Storage<'r>> {
        let boundary = req.content_type()
            .ok_or(multer::Error::NoMultipart)?
            .param("boundary")
            .ok_or(multer::Error::NoBoundary)?;

        let form_limit = req.limits()
            .get("data-form")
            .unwrap_or(Limits::DATA_FORM);

        let mp = Multipart::with_reader(data.open(form_limit), boundary);
        let buffer = local_cache!(req, Buffer::new());
        Ok(Storage::MultiPart(buffer, mp))
    }
}

#[crate::async_trait]
impl<'r, T: FromForm<'r> + 'r> FromData<'r> for Form<T> {
    type Error = Errors<'r>;

    async fn from_data(req: &'r Request<'_>, data: Data) -> Outcome<Self, Self::Error> {
        let storage = match req.content_type() {
            Some(c) if c.is_form() => Storage::string(req, data).await,
            Some(c) if c.is_form_data() => Storage::multipart(req, data).await,
            _ => return Outcome::Forward(data),
        };

        let storage = match storage {
            Ok(storage) => storage,
            Err(e) => return Outcome::Failure((e.status(), e))
        };

        match parse_form(req, storage, Options::Lenient).await {
            Ok(value) => Outcome::Success(Form(value)),
            Err(e) => Outcome::Failure((e.status(), e)),
        }
    }
}

#[derive(Debug)]
pub struct Strict<T>(T);

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

impl<T> Strict<T> {
    pub fn into_inner(self) -> T {
        self.0
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
