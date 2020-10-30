// Rocket sometimes generates mangled identifiers that activate the
// non_snake_case lint. We deny the lint in this test to ensure that
// code generation uses #[allow(non_snake_case)] in the appropriate places.
#![deny(non_snake_case)]

#[macro_use] extern crate rocket;

use std::path::PathBuf;

use rocket::request::Request;
use rocket::http::ext::Normalize;
use rocket::local::blocking::Client;
use rocket::data::{self, Data, FromData};
use rocket::http::{Status, RawStr, ContentType};

// Use all of the code generation available at once.

#[derive(FromForm, UriDisplayQuery)]
struct Inner<'r> {
    field: &'r str
}

struct Simple(String);

#[async_trait]
impl<'r> FromData<'r> for Simple {
    type Error = std::io::Error;

    async fn from_data(req: &'r Request<'_>, data: Data) -> data::Outcome<Self, Self::Error> {
        String::from_data(req, data).await.map(Simple)
    }
}

#[post(
    "/<a>/<name>/name/<path..>?sky=blue&<sky>&<query..>",
    format = "json",
    data = "<simple>",
    rank = 138
)]
fn post1(
    sky: usize,
    name: &str,
    a: String,
    query: Inner<'_>,
    path: PathBuf,
    simple: Simple,
) -> String {
    let string = format!("{}, {}, {}, {}, {}, {}",
        sky, name, a, query.field, path.normalized_str(), simple.0);

    let uri = uri!(post1: a, name, path, sky, query);

    format!("({}) ({})", string, uri.to_string())
}

#[route(
    POST,
    path = "/<a>/<name>/name/<path..>?sky=blue&<sky>&<query..>",
    format = "json",
    data = "<simple>",
    rank = 138
)]
fn post2(
    sky: usize,
    name: &str,
    a: String,
    query: Inner<'_>,
    path: PathBuf,
    simple: Simple,
) -> String {
    let string = format!("{}, {}, {}, {}, {}, {}",
        sky, name, a, query.field, path.normalized_str(), simple.0);

    let uri = uri!(post2: a, name, path, sky, query);

    format!("({}) ({})", string, uri.to_string())
}

#[allow(dead_code)]
#[post("/<_unused_param>?<_unused_query>", data="<_unused_data>")]
fn test_unused_params(_unused_param: String, _unused_query: String, _unused_data: Data) {
}

#[test]
fn test_full_route() {
    let rocket = rocket::ignite()
        .mount("/1", routes![post1])
        .mount("/2", routes![post2]);

    let client = Client::tracked(rocket).unwrap();

    let a = RawStr::new("A%20A");
    let name = RawStr::new("Bob%20McDonald");
    let path = "this/path/here";
    let sky = 777;
    let query = "field=inside";
    let simple = "data internals";

    let path_part = format!("/{}/{}/name/{}", a, name, path);
    let query_part = format!("?sky={}&sky=blue&{}", sky, query);
    let uri = format!("{}{}", path_part, query_part);
    let expected_uri = format!("{}?sky=blue&sky={}&{}", path_part, sky, query);

    let response = client.post(&uri).body(simple).dispatch();
    assert_eq!(response.status(), Status::NotFound);

    let response = client.post(format!("/1{}", uri)).body(simple).dispatch();
    assert_eq!(response.status(), Status::NotFound);

    let response = client
        .post(format!("/1{}", uri))
        .header(ContentType::JSON)
        .body(simple)
        .dispatch();

    assert_eq!(response.into_string().unwrap(), format!("({}, {}, {}, {}, {}, {}) ({})",
            sky, name.percent_decode().unwrap(), "A A", "inside", path, simple, expected_uri));

    let response = client.post(format!("/2{}", uri)).body(simple).dispatch();
    assert_eq!(response.status(), Status::NotFound);

    let response = client
        .post(format!("/2{}", uri))
        .header(ContentType::JSON)
        .body(simple)
        .dispatch();

    assert_eq!(response.into_string().unwrap(), format!("({}, {}, {}, {}, {}, {}) ({})",
            sky, name.percent_decode().unwrap(), "A A", "inside", path, simple, expected_uri));
}

mod scopes {
    mod other {
        #[get("/world")]
        pub fn world() -> &'static str {
            "Hello, world!"
        }
    }

    #[get("/hello")]
    pub fn hello() -> &'static str {
        "Hello, outside world!"
    }

    use other::world;

    fn _rocket() -> rocket::Rocket {
        rocket::ignite().mount("/", rocket::routes![hello, world])
    }
}
