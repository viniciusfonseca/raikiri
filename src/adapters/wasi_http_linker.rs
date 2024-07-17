// NOTE: for some reason, these functions aren't public in the `wasmtime-wasi-http` crate. This is a copy paste from the crate source so I can setup the linker properly

use wasmtime_wasi_http::{bindings, WasiHttpImpl, WasiHttpView};

fn type_annotate_http<T, F>(val: F) -> F
where
    F: Fn(&mut T) -> WasiHttpImpl<&mut T>,
{
    val
}
fn type_annotate_wasi<T, F>(val: F) -> F
where
    F: Fn(&mut T) -> wasmtime_wasi::WasiImpl<&mut T>,
{
    val
}

pub fn add_to_linker_sync<T>(l: &mut wasmtime::component::Linker<T>) -> anyhow::Result<()>
where
    T: WasiHttpView + wasmtime_wasi::WasiView,
{
    let closure = type_annotate_wasi::<T, _>(|t| wasmtime_wasi::WasiImpl(t));

    wasmtime_wasi::bindings::clocks::wall_clock::add_to_linker_get_host(l, closure)?;
    wasmtime_wasi::bindings::clocks::monotonic_clock::add_to_linker_get_host(l, closure)?;
    wasmtime_wasi::bindings::sync::io::poll::add_to_linker_get_host(l, closure)?;
    wasmtime_wasi::bindings::sync::io::streams::add_to_linker_get_host(l, closure)?;
    wasmtime_wasi::bindings::io::error::add_to_linker_get_host(l, closure)?;
    wasmtime_wasi::bindings::cli::stdin::add_to_linker_get_host(l, closure)?;
    wasmtime_wasi::bindings::cli::stdout::add_to_linker_get_host(l, closure)?;
    wasmtime_wasi::bindings::cli::stderr::add_to_linker_get_host(l, closure)?;
    wasmtime_wasi::bindings::random::random::add_to_linker_get_host(l, closure)?;

    add_only_http_to_linker_sync(l)?;

    Ok(())
}

pub fn add_only_http_to_linker_sync<T>(l: &mut wasmtime::component::Linker<T>) -> anyhow::Result<()>
where
    T: WasiHttpView,
{
    let closure = type_annotate_http::<T, _>(|t| WasiHttpImpl(t));

    bindings::http::outgoing_handler::add_to_linker_get_host(l, closure)?;
    bindings::http::types::add_to_linker_get_host(l, closure)?;

    Ok(())
}