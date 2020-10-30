#[cfg(any(test, doctest))] rocket::internal_guide_tests!("../guide/*.md");
#[cfg(any(test, doctest))] rocket::internal_guide_tests!("../../../README.md");

#[macro_export]
macro_rules! map {
    ($($key:expr => $value:expr),* $(,)?) => ({
        let mut map = std::collections::HashMap::new();
        $(map.insert($key.into(), $value.into());)*
        map
    });
}

#[macro_export]
macro_rules! bmap {
    ($($key:expr => $value:expr),* $(,)?) => ({
        let mut map = std::collections::BTreeMap::new();
        $(map.insert($key.into(), $value.into());)*
        map
    });
}

#[macro_export]
macro_rules! assert_form_parses {
    ($T:ty, $form:expr => $value:expr) => (
        let v = rocket::form::Form::<$T>::parse_url_encoded($form).unwrap();
        assert_eq!(v, $value, "{}", $form);
    );

    ($T:ty, $($form:expr => $value:expr),+ $(,)?) => (
        $(assert_form_parses!($T, $form => $value);)+
    );

    ($T:ty, $($form:expr),+ $(,)? => $value:expr) => (
        $(assert_form_parses!($T, $form => $value);)+
    );
}

#[macro_export]
macro_rules! assert_not_form_parses {
    ($T:ty, $($form:expr),* $(,)?) => ($(
        rocket::form::Form::<$T>::parse_url_encoded($form).unwrap_err();
    )*);
}

#[macro_export]
macro_rules! assert_form_parses_ok {
    ($T:ty, $($form:expr),* $(,)?) => ($(
        rocket::form::Form::<$T>::parse_url_encoded($form).expect("form to parse");
    )*);
}
