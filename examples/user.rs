extern crate hyper;
extern crate path_router;

use hyper::rt::Future;
use hyper::service::service_fn_ok;
use hyper::{Body, Request, Response, Server};
use path_router::{Match, Router};
use std::collections::BTreeMap;
use std::sync::Arc;

type Handler = fn(Request<Body>, BTreeMap<String, String>) -> Body;

fn handler_get_user_info(_req: Request<Body>, params: BTreeMap<String, String>) -> Body {
    let uid = params["id"].clone();
    Body::from(uid)
}

fn handler_add_user(_req: Request<Body>, _params: BTreeMap<String, String>) -> Body {
    Body::from("ok")
}

fn handler_get_user_attributes(_req: Request<Body>, params: BTreeMap<String, String>) -> Body {
    let result = params["attrs"].split("/").collect::<Vec<&str>>().join(" ");
    Body::from(result)
}

fn handler_notfound(_req: Request<Body>) -> Body {
    Body::from("notfound")
}

fn main() {
    let addr = ([127, 0, 0, 1], 3000).into();

    let mut router = Router::<Handler>::default();
    // curl localhost:3000/user/123
    router.add("/GET/user/:id", handler_get_user_info).unwrap();
    // curl -X POST localhost:3000/user
    router.add("/POST/user", handler_add_user).unwrap();
    // curl localhost:3000/user/123/name/gender
    router
        .add("/GET/user/:id/*attrs", handler_get_user_attributes)
        .unwrap();

    let router = Arc::new(router);

    let new_svc = move || {
        let router = Arc::clone(&router);

        service_fn_ok(move |req| {
            let path = format!("/{}{}", req.method().as_str(), req.uri().path());
            match router.recognize(path.as_str()) {
                Some(Match { data, params }) => Response::new(data(req, params)),
                None => Response::new(handler_notfound(req)),
            }
        })
    };

    let server = Server::bind(&addr)
        .serve(new_svc)
        .map_err(|e| eprintln!("server error: {}", e));

    hyper::rt::run(server);
}
