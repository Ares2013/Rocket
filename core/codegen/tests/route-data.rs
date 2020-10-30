#[macro_use]extern crate rocket;

use rocket::{Request, Data};
use rocket::local::blocking::Client;
use rocket::data::{self, FromData};
use rocket::http::ContentType;
use rocket::form::Form;

// Test that the data parameters works as expected.

#[derive(FromForm)]
struct Inner {
    field: String
}

struct Simple(String);

#[async_trait]
impl<'r> FromData<'r> for Simple {
    type Error = std::io::Error;

    async fn from_data(req: &'r Request<'_>, data: Data) -> data::Outcome<Self, Self::Error> {
        String::from_data(req, data).await.map(Simple)
    }
}

#[post("/f", data = "<form>")]
fn form(form: Form<Inner>) -> String { form.into_inner().field }

#[post("/s", data = "<simple>")]
fn simple(simple: Simple) -> String { simple.0 }

#[test]
fn test_data() {
    let rocket = rocket::ignite().mount("/", routes![form, simple]);
    let client = Client::tracked(rocket).unwrap();

    let response = client.post("/f")
        .header(ContentType::Form)
        .body("field=this%20is%20here")
        .dispatch();

    assert_eq!(response.into_string().unwrap(), "this is here");

    let response = client.post("/s").body("this is here").dispatch();
    assert_eq!(response.into_string().unwrap(), "this is here");

    let response = client.post("/s").body("this%20is%20here").dispatch();
    assert_eq!(response.into_string().unwrap(), "this%20is%20here");
}
