use std::ops::Deref;
use std::borrow::Cow;

use ref_cast::RefCast;

use crate::http::RawStr;

#[repr(transparent)]
#[derive(RefCast)]
pub struct Name(str);

impl Name {
    pub fn new<S: AsRef<str> + ?Sized>(string: &S) -> &Name {
        Name::ref_cast(string.as_ref())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn keys(&self) -> impl Iterator<Item = &Key> {
        struct Keys<'v>(NameView<'v>);

        impl<'v> Iterator for Keys<'v> {
            type Item = &'v Key;

            fn next(&mut self) -> Option<Self::Item> {
                if self.0.is_terminal() {
                    return None;
                }

                let key = self.0.key_lossy();
                self.0.shift();
                Some(key)
            }
        }

        Keys(NameView::new(self))
    }

    pub fn prefixes(&self) -> impl Iterator<Item = &Name> {
        struct Prefixes<'v>(NameView<'v>);

        impl<'v> Iterator for Prefixes<'v> {
            type Item = &'v Name;

            fn next(&mut self) -> Option<Self::Item> {
                if self.0.is_terminal() {
                    return None;
                }

                let name = self.0.as_name();
                self.0.shift();
                Some(name)
            }
        }

        Prefixes(NameView::new(self))
    }
}

impl serde::Serialize for Name {
    fn serialize<S>(&self, ser: S) -> Result<S::Ok, S::Error>
        where S: serde::Serializer
    {
        self.0.serialize(ser)
    }
}

impl<'de: 'a, 'a> serde::Deserialize<'de> for &'a Name {
    fn deserialize<D>(de: D) -> Result<Self, D::Error>
        where D: serde::Deserializer<'de>
    {
        <&'a str as serde::Deserialize<'de>>::deserialize(de).map(Name::new)
    }
}

impl<'a, S: AsRef<str> + ?Sized> From<&'a S> for &'a Name {
    #[inline]
    fn from(string: &'a S) -> Self {
        Name::new(string)
    }
}

impl Deref for Name {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<I: core::slice::SliceIndex<str, Output=str>> core::ops::Index<I> for Name {
    type Output = Name;

    #[inline]
    fn index(&self, index: I) -> &Self::Output {
        self.0[index].into()
    }
}

impl PartialEq for Name {
    fn eq(&self, other: &Self) -> bool {
        self.keys().eq(other.keys())
    }
}

impl PartialEq<str> for Name {
    fn eq(&self, other: &str) -> bool {
        self == Name::new(other)
    }
}

impl PartialEq<Name> for str {
    fn eq(&self, other: &Name) -> bool {
        Name::new(self) == other
    }
}

impl PartialEq<&str> for Name {
    fn eq(&self, other: &&str) -> bool {
        self == Name::new(other)
    }
}

impl PartialEq<Name> for &str {
    fn eq(&self, other: &Name) -> bool {
        Name::new(self) == other
    }
}

impl AsRef<Name> for str {
    fn as_ref(&self) -> &Name {
        Name::new(self)
    }
}

impl AsRef<Name> for RawStr {
    fn as_ref(&self) -> &Name {
        Name::new(self)
    }
}

impl Eq for Name { }

impl std::hash::Hash for Name {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.keys().for_each(|k| k.0.hash(state))
    }
}

impl std::fmt::Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::fmt::Debug for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[repr(transparent)]
#[derive(RefCast, Debug, PartialEq, Eq, Hash)]
pub struct Key(str);

impl Key {
    pub fn new<S: AsRef<str> + ?Sized>(string: &S) -> &Key {
        Key::ref_cast(string.as_ref())
    }

    pub fn as_str(&self) -> &str {
        &*self
    }

    pub fn indices(&self) -> impl Iterator<Item = &str> {
        self.split(':')
    }
}

impl Deref for Key {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl serde::Serialize for Key {
    fn serialize<S>(&self, ser: S) -> Result<S::Ok, S::Error>
        where S: serde::Serializer
    {
        self.0.serialize(ser)
    }
}

impl<'de: 'a, 'a> serde::Deserialize<'de> for &'a Key {
    fn deserialize<D>(de: D) -> Result<Self, D::Error>
        where D: serde::Deserializer<'de>
    {
        <&'a str as serde::Deserialize<'de>>::deserialize(de).map(Key::new)
    }
}

impl<I: core::slice::SliceIndex<str, Output=str>> core::ops::Index<I> for Key {
    type Output = Key;

    #[inline]
    fn index(&self, index: I) -> &Self::Output {
        self.0[index].into()
    }
}

impl PartialEq<str> for Key {
    fn eq(&self, other: &str) -> bool {
        self == Key::new(other)
    }
}

impl PartialEq<Key> for str {
    fn eq(&self, other: &Key) -> bool {
        Key::new(self) == other
    }
}

impl<'a, S: AsRef<str> + ?Sized> From<&'a S> for &'a Key {
    #[inline]
    fn from(string: &'a S) -> Self {
        Key::new(string)
    }
}

impl AsRef<Key> for str {
    fn as_ref(&self) -> &Key {
        Key::new(self)
    }
}

impl AsRef<Key> for RawStr {
    fn as_ref(&self) -> &Key {
        Key::new(self)
    }
}

impl std::fmt::Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Copy, Clone)]
pub struct NameView<'v> {
    name: &'v Name,
    start: usize,
    end: usize,
}

impl<'v> NameView<'v> {
    pub fn new<N: Into<&'v Name>>(name: N) -> Self {
        let mut view = NameView { name: name.into(), start: 0, end: 0 };
        view.shift();
        view
    }

    fn is_terminal(&self) -> bool {
        self.start == self.name.len()
    }

    pub fn parent(&self) -> Option<&'v Name> {
        if self.start > 0 {
            Some(&self.name[..self.start])
        } else {
            None
        }
    }

    pub fn as_name(&self) -> &'v Name {
        &self.name[..self.end]
    }

    pub fn shift(&mut self) {
        const START_DELIMS: &'static [char] = &['.', '['];

        let string = &self.name[self.end..];
        let bytes = string.as_bytes();
        let shift = match bytes.get(0) {
            None | Some(b'=') => 0,
            Some(b'[') => match string[1..].find(&[']', '.'][..]) {
                Some(j) => match string[1..].as_bytes()[j] {
                    b']' => j + 2,
                    _ => j + 1,
                }
                None => bytes.len(),
            }
            Some(b'.') => match string[1..].find(START_DELIMS) {
                Some(j) => j + 1,
                None => bytes.len(),
            },
            _ => match string.find(START_DELIMS) {
                Some(j) => j,
                None => bytes.len()
            }
        };

        debug_assert!(self.end + shift <= self.name.len());
        *self = NameView {
            name: self.name,
            start: self.end,
            end: self.end + shift,
        };
    }

    /// Allows empty keys.
    pub fn key_lossy(&self) -> &'v Key {
        let view = &self.name[self.start..self.end];
        let key = match view.as_bytes().get(0) {
            Some(b'.') => &view[1..],
            Some(b'[') if view.ends_with(']') => &view[1..view.len() - 1],
            _ => view
        };

        key.0.into()
    }

    /// Does not allow empty keys.
    pub fn key(&self) -> Option<&'v Key> {
        let lossy_key = self.key_lossy();
        if lossy_key.is_empty() {
            return None;
        }

        Some(lossy_key)
    }

    pub fn source(&self) -> &'v Name {
        self.name
    }
}

impl std::fmt::Debug for NameView<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_name().fmt(f)
    }
}

impl std::fmt::Display for NameView<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_name().fmt(f)
    }
}

impl<'a, 'b> PartialEq<NameView<'b>> for NameView<'a> {
    fn eq(&self, other: &NameView<'b>) -> bool {
        self.as_name() == other.as_name()
    }
}

impl<B: PartialEq<Name>> PartialEq<B> for NameView<'_> {
    fn eq(&self, other: &B) -> bool {
        other == self.as_name()
    }
}

impl Eq for NameView<'_> {  }

impl std::hash::Hash for NameView<'_> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_name().hash(state)
    }
}

impl std::borrow::Borrow<Name> for NameView<'_> {
    fn borrow(&self) -> &Name {
        self.as_name()
    }
}

#[derive(Clone)]
pub struct NameViewCow<'v> {
    left: &'v Name,
    right: Cow<'v, str>,
}

impl crate::http::ext::IntoOwned for NameViewCow<'_> {
    type Owned = NameViewCow<'static>;

    fn into_owned(self) -> Self::Owned {
        let right = match (self.left, self.right) {
            (l, Cow::Owned(r)) if l.is_empty() => Cow::Owned(r),
            (l, r) if l.is_empty() => r.to_string().into(),
            (l, r) if r.is_empty() => l.to_string().into(),
            (l, r) => format!("{}.{}", l, r).into(),
        };

        NameViewCow { left: "".into(), right }
    }
}

impl<'v> NameViewCow<'v> {
    pub fn is_empty(&self) -> bool {
        self.left.is_empty() && self.right.is_empty()
    }

    fn split(&self) -> (&Name, &Name) {
        (self.left, Name::new(&self.right))
    }

    pub fn keys(&self) -> impl Iterator<Item = &Key> {
        let (left, right) = self.split();
        left.keys().chain(right.keys())
    }
}

impl serde::Serialize for NameViewCow<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: serde::Serializer
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'v> From<NameView<'v>> for NameViewCow<'v> {
    fn from(nv: NameView<'v>) -> Self {
        NameViewCow { left: nv.as_name(), right: Cow::Borrowed("") }
    }
}

impl<'v> From<(Option<&'v Name>, &'v str)> for NameViewCow<'v> {
    fn from((prefix, suffix): (Option<&'v Name>, &'v str)) -> Self {
        match prefix {
            Some(left) => NameViewCow { left, right: suffix.into() },
            None => NameViewCow { left: "".into(), right: suffix.into() }
        }
    }
}

impl<'v> From<(&'v Name, &'v str)> for NameViewCow<'v> {
    fn from((prefix, suffix): (&'v Name, &'v str)) -> Self {
        NameViewCow::from((Some(prefix), suffix))
    }
}

impl std::fmt::Debug for NameViewCow<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"")?;

        let (left, right) = self.split();
        if !left.is_empty() { write!(f, "{}", left.escape_debug())? }
        if !right.is_empty() {
            if !left.is_empty() { f.write_str(".")?; }
            write!(f, "{}", right.escape_debug())?;
        }

        write!(f, "\"")
    }
}

impl std::fmt::Display for NameViewCow<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (left, right) = self.split();
        if !left.is_empty() { left.fmt(f)?; }
        if !right.is_empty() {
            if !left.is_empty() { f.write_str(".")?; }
            right.fmt(f)?;
        }

        Ok(())
    }
}

impl PartialEq for NameViewCow<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.keys().eq(other.keys())
    }
}

impl<N: AsRef<Name> + ?Sized> PartialEq<N> for NameViewCow<'_> {
    fn eq(&self, other: &N) -> bool {
        self.keys().eq(other.as_ref().keys())
    }
}

impl PartialEq<Name> for NameViewCow<'_> {
    fn eq(&self, other: &Name) -> bool {
        self.keys().eq(other.keys())
    }
}

impl PartialEq<NameViewCow<'_>> for Name {
    fn eq(&self, other: &NameViewCow<'_>) -> bool {
        self.keys().eq(other.keys())
    }
}

impl PartialEq<NameViewCow<'_>> for str {
    fn eq(&self, other: &NameViewCow<'_>) -> bool {
        Name::new(self) == other
    }
}

impl PartialEq<NameViewCow<'_>> for &str {
    fn eq(&self, other: &NameViewCow<'_>) -> bool {
        Name::new(self) == other
    }
}

impl Eq for NameViewCow<'_> { }

impl std::hash::Hash for NameViewCow<'_> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.keys().for_each(|k| k.0.hash(state))
    }
}

impl indexmap::Equivalent<Name> for NameViewCow<'_> {
    fn equivalent(&self, key: &Name) -> bool {
        self.keys().eq(key.keys())
    }
}

impl indexmap::Equivalent<NameViewCow<'_>> for Name {
    fn equivalent(&self, key: &NameViewCow<'_>) -> bool {
        self.keys().eq(key.keys())
    }
}
