#![feature(in_band_lifetimes)]

use casbin::prelude::*;
use rocket::{
    fairing::Fairing,
    http::Status,
    request::{Outcome, FromRequest},
    Request,
};
use std::sync::{Arc, RwLock};

/// Trait implemented by rocket fairings to authorize incoming requests.
pub trait CasbinMiddleware: Fairing {
    /// get enforce rvals from request.
    /// the values returned are usually: [sub, obj, act], depend on your model.
    fn casbin_vals(&self, req: &'a Request<'r>) -> Vec<String>;

    fn cached_enforcer(&self) -> Arc<RwLock<CachedEnforcer>>;

    /// authorize request, and add result to request
    fn enforce(&self, req: &'a Request<'r>) {
        let vals = self.casbin_vals(req);
        let vals = (&vals).into_iter().map(|v| v).collect::<Vec<&String>>();

        let cloned_enforcer = self.cached_enforcer();
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

/// A request guard that handle authorization result.
/// CasbinGuard usually appear as arguments in a route handler.
/// 
/// Example
/// ```ignore
/// #[get("/book/1")]
/// fn book(_g: CasbinGuard) { /* ... */ }
/// ```
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
