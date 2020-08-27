#![feature(proc_macro_hygiene, decl_macro)]
#![feature(in_band_lifetimes)]

use rocket::{
    get,
    fairing::{Info, Kind, Fairing},
    request::{Request},
    routes,
    Data,
};

use casbin::prelude::*;
use std::sync::{Arc, RwLock};
use rocket_casbin_auth::{CasbinGuard, CasbinMiddleware};

pub struct CasbinFairing {
    enforcer: Arc<RwLock<CachedEnforcer>>,
}

impl CasbinFairing {
    pub fn new<M: TryIntoModel, A: TryIntoAdapter>(m: M, a: A) -> CasbinFairing {
        let mut rt = tokio::runtime::Runtime::new().unwrap();
        match rt.block_on(casbin::CachedEnforcer::new(m, a)) {
            Ok(e) => CasbinFairing {
                enforcer: Arc::new(RwLock::new(e)),
            },
            Err(_) => panic!("CasbinFairing build failed"),
        }
    }
}

impl CasbinMiddleware for CasbinFairing {
    fn casbin_vals<'a>(&self, req: &Request<'_>) -> Vec<String> {
        let path = req.uri().path().to_owned();
        let sub = match req.cookies().get("name") {
            Some(cookie) => cookie.value().to_owned(),
            _ => "".to_owned(),
        };
        let method = req.method().as_str().to_owned();
        vec![sub, path, method]
    }

    fn cached_enforcer(&self) -> Arc<RwLock<CachedEnforcer>> {
        self.enforcer.clone()
    }
}

impl Fairing for CasbinFairing {
    fn info(&self) -> Info {
        Info {
            name: "Casbin Fairing",
            kind: Kind::Request,
        }
    }

    fn on_request(&self, req: &mut Request<'r>, _: &Data) {
        self.enforce(req);
    }
}

#[get("/pen")]
pub fn pen(_g: CasbinGuard) -> &'static str {
    "pen"
}

#[get("/book/1")]
pub fn book(_g: CasbinGuard) -> &'static str {
    "book"
}

fn rocket() -> rocket::Rocket {
    rocket::ignite()
        .attach(CasbinFairing::new("examples/model.conf", "examples/role_policy.csv"))
        .mount("/", routes![pen, book])
}

fn main() {
    rocket().launch();
}

#[cfg(test)]
mod test {
    use super::rocket;
    use rocket::local::Client;
    use rocket::http::Cookie;
    use rocket::http::Status;

    #[test]
    fn test_hello() {
        let client = Client::new(rocket()).unwrap();

        let mut response = client.get("/book/1").cookie(Cookie::new("name", "alice")).dispatch();
        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.body_string(), Some("book".into()));

        let mut response = client.get("/book/1").cookie(Cookie::new("name", "bob")).dispatch();
        assert_eq!(response.status(), Status::Forbidden);

        let mut response = client.get("/pen").cookie(Cookie::new("name", "alice")).dispatch();
        assert_eq!(response.status(), Status::Forbidden);

        let mut response = client.get("/pen").cookie(Cookie::new("name", "bob")).dispatch();
        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.body_string(), Some("pen".into()));
    }
}