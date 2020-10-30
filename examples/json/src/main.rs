#[macro_use] extern crate rocket;
#[macro_use] extern crate rocket_contrib;

#[cfg(test)] mod tests;

use std::sync::Mutex;
use std::collections::HashMap;
use std::borrow::Cow;

use rocket::State;
use rocket_contrib::json::{Json, JsonValue};

use serde::{Serialize, Deserialize};

// The type to represent the ID of a message.
type ID = usize;

// We're going to store all of the messages here. No need for a DB.
type MessageMap = Mutex<HashMap<ID, String>>;

#[derive(Serialize, Deserialize)]
struct Message<'r> {
    id: Option<ID>,
    contents: Cow<'r, str>
}

#[post("/<id>", format = "json", data = "<message>")]
fn new(id: ID, message: Json<Message>, map: State<'_, MessageMap>) -> JsonValue {
    let mut hashmap = map.lock().expect("map lock.");
    if hashmap.contains_key(&id) {
        json!({
            "status": "error",
            "reason": "ID exists. Try put."
        })
    } else {
        hashmap.insert(id, message.into_inner().contents.into());
        json!({ "status": "ok" })
    }
}

#[put("/<id>", format = "json", data = "<message>")]
fn update(id: ID, message: Json<Message>, map: State<'_, MessageMap>) -> Option<JsonValue> {
    let mut hashmap = map.lock().unwrap();
    if hashmap.contains_key(&id) {
        hashmap.insert(id, message.into_inner().contents.into());
        Some(json!({ "status": "ok" }))
    } else {
        None
    }
}

#[get("/<id>", format = "json")]
fn get<'r>(id: ID, map: State<'_, MessageMap>) -> Option<Json<Message<'static>>> {
    let hashmap = map.lock().unwrap();
    let contents = hashmap.get(&id)?;
    Some(Json(Message {
        id: Some(id),
        contents: contents.clone().into()
    }))
}

#[get("/echo", data = "<msg>")]
fn echo<'r>(msg: Json<Message<'r>>) -> Cow<'r, str> {
    msg.into_inner().contents
}

#[catch(404)]
fn not_found() -> JsonValue {
    json!({
        "status": "error",
        "reason": "Resource was not found."
    })
}

#[launch]
fn rocket() -> rocket::Rocket {
    rocket::ignite()
        .mount("/message", routes![new, update, get, echo])
        .register(catchers![not_found])
        .manage(Mutex::new(HashMap::<ID, String>::new()))
}
