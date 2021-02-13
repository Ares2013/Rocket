#[macro_use]extern crate rocket;

use rocket::form::{Form, FromForm, FromFormField, Context};
use rocket::data::TempFile;

use rocket_contrib::serve::{StaticFiles, crate_relative};
use rocket_contrib::templates::Template;

#[derive(Debug, FromForm)]
struct Password<'v> {
    #[field(validate = len(6..))]
    first: &'v str,
    #[field(validate = eq(self.first))]
    second: &'v str,
}

#[derive(Debug, FromFormField)]
enum Rights {
    Public,
    Reserved,
    Exclusive,
}

#[derive(Debug, FromFormField)]
enum Category {
    Biology,
    Chemistry,
    Physics,
    #[field(value = "CS")]
    ComputerScience,
}

#[derive(Debug, FromForm)]
struct Submission<'v> {
    #[field(validate = len(1..))]
    title: String,
    date: time::Date,
    #[field(validate = len(1..=250))]
    r#abstract: String,
    #[field(validate = ext("pdf"))]
    file: TempFile<'v>,
    #[field(validate = len(1..))]
    category: Vec<Category>,
    rights: Rights,
    ready: bool,
}

#[derive(Debug, FromForm)]
struct Account<'v> {
    #[field(validate = len(1..))]
    name: String,
    password: Password<'v>,
    #[field(validate = contains('@'))]
    #[field(validate = omits(self.password.first))]
    email: String,
}

#[derive(Debug, FromForm)]
struct Submit<'v> {
    account: Account<'v>,
    submission: Submission<'v>,
    submissions: Vec<Submission<'v>>,
}

#[get("/")]
fn index<'r>() -> Template {
    Template::render("index", &Context::default())
}

// #[post("/", data = "<form>")]
// fn submit<'r>(form: Contextual<Form<Submit<'r>, Context<'r>>>) -> Template {
//     match form.into_inner() {
//         Ok(_submission) => {
//             // Do something with submission...render a different template?
//             index()
//         }
//         Err(ctxt) => {
//             dbg!(&ctxt);
//             Template::render("index", &ctxt)
//         }
//     }
// }

use rocket::form::Errors;
use rocket::response::Redirect;

#[post("/", data = "<form>")]
fn submit<'r>(form: Result<Form<Submit<'r>>, Errors<'r>>) -> Redirect {
    if let Err(e) = form {
        for e in e {
            eprintln!("error: {:?}", e);
        }
    }

    Redirect::to(uri!(index))

    // match form.into_inner() {
    //     Ok(_submission) => {
    //         // Do something with submission...render a different template?
    //         index()
    //     }
    //     Err(ctxt) => {
    //         dbg!(&ctxt);
    //         Template::render("index", &ctxt)
    //     }
    // }
}

#[launch]
fn rocket() -> rocket::Rocket {
    rocket::ignite()
        .mount("/", routes![index, submit])
        .attach(Template::fairing())
        .mount("/", StaticFiles::from(crate_relative!("/static")))
}
