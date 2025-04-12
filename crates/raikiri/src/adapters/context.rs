use wasmtime_wasi_http::{body::HyperOutgoingBody, types::{HostFutureIncomingResponse, OutgoingRequestConfig}, HttpResult};

use crate::domain::raikiri_env::RaikiriEnvironment;

pub trait RaikiriContext {
    fn call_stack(&self) -> &Vec<String>;
    fn environment(&self) -> &RaikiriEnvironment;
    fn handle_http(&self, request: hyper::Request<HyperOutgoingBody>,
        config: OutgoingRequestConfig,
    ) -> HttpResult<HostFutureIncomingResponse>; 
}