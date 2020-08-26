#![feature(in_band_lifetimes)]

use casbin::prelude::*;
use rocket::{
    fairing::Fairing,
    http::Status,
    request::{Outcome, FromRequest},
    Request,
};
use std::sync::{Arc, RwLock};

pub trait CasbinMiddleware: Fairing {
    fn get_casbin_vals(&self, req: &'a Request<'r>) -> Vec<String>;

    fn get_cached_enforcer(&self) -> Arc<RwLock<CachedEnforcer>>;

    fn enforce(&self, req: &'a Request<'r>) {
        let vals = self.get_casbin_vals(req);
        let vals = (&vals).into_iter().map(|v| v).collect::<Vec<&String>>();

        let cloned_enforcer = self.get_cached_enforcer();
        let mut lock_enforcer = cloned_enforcer.write().unwrap();
        match lock_enforcer.enforce_mut(&vals) {
            Ok(true) => {
                req.local_cache(|| CasbinGuard(Some(Status::Ok)));
            }
            Ok(false) => {
                req.local_cache(|| CasbinGuard(Some(Status::Forbidden)));
            }
            Err(_) => {
                req.local_cache(|| CasbinGuard(Some(Status::BadGateway)));
            }
        }
    }
}

pub struct CasbinGuard(Option<Status>);

impl<'a, 'r> FromRequest<'a, 'r> for CasbinGuard {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> Outcome<CasbinGuard, ()> {
        match *request.local_cache(|| CasbinGuard(Status::from_code(0))) {
            CasbinGuard(Some(Status::Ok)) => {
                Outcome::Success(CasbinGuard(Some(Status::Ok)))
            }
            CasbinGuard(Some(err_status)) => Outcome::Failure((err_status, ())),
            _ => Outcome::Failure((Status::BadGateway, ())),
        }
    }
}
