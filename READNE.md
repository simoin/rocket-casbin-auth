# Rocket Casbin Middleware
Casbin intergration for actix framework

## Usage

```rust
rocket_casbin_auth = "0.1.0"
```

## Guide

with the Rocket [Fairing Guide](https://rocket.rs/v0.4/guide/fairings/), we need to use [Fairing](https://api.rocket.rs/v0.4/rocket/fairing/trait.Fairing.html) trait for authentication or authorization with casbin. 

So you need to implement `CasbinMiddleware` and `Fairing`.
 
example:
```rust
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
    fn get_casbin_vals<'a>(&self, req: &Request<'_>) -> Vec<String> {
        let path = req.uri().path().to_owned();
        let sub = match req.cookies().get("name") {
            Some(cookie) => cookie.value().to_owned(),
            _ => "".to_owned(),
        };
        let method = req.method().as_str().to_owned();
        vec![sub, path, method]
    }

    fn get_cached_enforcer(&self) -> Arc<RwLock<CachedEnforcer>> {
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
```