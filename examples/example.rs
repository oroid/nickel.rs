#![feature(core, io, net)]

extern crate "rustc-serialize" as rustc_serialize;
extern crate nickel;
#[macro_use] extern crate nickel_macros;

use nickel::status::StatusCode::{self, NotFound, BadRequest};
use nickel::{
    Nickel, NickelError, ErrorWithStatusCode, Continue, Halt, Request,
    QueryString, JsonBody, StaticFilesHandler, HttpRouter, Action
};
use std::net::IpAddr;
use std::collections::BTreeMap;
use std::io::Write;
use rustc_serialize::json::{Json, ToJson};

#[derive(RustcDecodable, RustcEncodable)]
struct Person {
    firstname: String,
    lastname:  String,
}

impl ToJson for Person {
    fn to_json(&self) -> Json {
        let mut map = BTreeMap::new();
        map.insert("first_name".to_string(), self.firstname.to_json());
        map.insert("last_name".to_string(), self.lastname.to_json());
        Json::Object(map)
    }
}

fn main() {
    let mut server = Nickel::new();

    // we would love to use a closure for the handler but it seems to be hard
    // to achieve with the current version of rust.

    //this is an example middleware function that just logs each request
    // middleware is optional and can be registered with `utilize`
    server.utilize(middleware! { |request|
        println!("logging request: {:?}", request.origin.uri);
    });

    let mut router = Nickel::router();

    // go to http://localhost:6767/user/4711 to see this route in action
    router.get("/user/:userid", middleware! { |request|
        format!("This is user: {}", request.param("userid"))
    });

    // go to http://localhost:6767/bar to see this route in action
    router.get("/bar", middleware!("This is the /bar handler"));

    // go to http://localhost:6767/some/crazy/route to see this route in action
    router.get("/some/*/route", middleware! {
        "This matches /some/crazy/route but not /some/super/crazy/route"
    });

    // go to http://localhost:6767/a/nice/route or http://localhost:6767/a/super/nice/route to see this route in action
    router.get("/a/**/route", middleware! {
        "This matches /a/crazy/route and also /a/super/crazy/route"
    });

    // try it with curl
    // curl 'http://localhost:6767/a/post/request' -H 'Content-Type: application/json;charset=UTF-8'  --data-binary $'{ "firstname": "John","lastname": "Connor" }'
    router.post("/a/post/request", middleware! { |request, response|
        let person = request.json_as::<Person>().unwrap();
        format!("Hello {} {}", person.firstname, person.lastname)
    });

    // go to http://localhost:6767/api/person/1 to see this route in action
    router.get("/api/person/1", middleware! {
        let person = Person {
            firstname: "Pea".to_string(),
            lastname: "Nut".to_string()
        };
        person.to_json()
    });

    // try calling http://localhost:6767/query?foo=bar
    router.get("/query", middleware! { |request|
        format!("Your foo values in the query string are: {:?}",
                request.query("foo", "This is only a default value!"))
    });

    // try calling http://localhost:6767/strict?state=valid
    // then try calling http://localhost:6767/strict?state=invalid
    router.get("/strict", middleware! { |request|
        if request.query("state", "invalid")[0].as_slice() != "valid" {
            (BadRequest, "Error Parsing JSON")
        } else {
            (StatusCode::Ok, "Congratulations on conforming!")
        }
    });

    server.utilize(router);

    // go to http://localhost:6767/thoughtram_logo_brain.png to see static file serving in action
    server.utilize(StaticFilesHandler::new("examples/assets/"));

    //this is how to overwrite the default error handler to handle 404 cases with a custom view
    fn custom_404<'a>(err: &mut NickelError, _req: &mut Request) -> Action {
        match err.kind {
            ErrorWithStatusCode(NotFound) => {
                // FIXME: Supportable?
                // response.content_type(MediaType::Html)
                //         .status_code(NotFound)
                //         .send("<h1>Call the police!<h1>");
                if let Some(ref mut res) = err.stream {
                    let _ = res.write_all(b"<h1>Call the police!</h1>");
                }
                Halt(())
            },
            _ => Continue(())
        }
    }

    // issue #20178
    let custom_handler: fn(&mut NickelError, &mut Request) -> Action = custom_404;

    server.handle_error(custom_handler);

    server.listen(IpAddr::new_v4(127, 0, 0, 1), 6767);
}
