# Raikiri


Raikiri is a open-source platform for running WebAssembly components in the server-side. It pre-compiles WebAssembly bytecode from the [Component Model](https://component-model.bytecodealliance.org/) using the [Wasmtime WASM runtime](https://wasmtime.dev/), stores the resulting assembly in local storage and invokes it whenever requested by the user. Components are by default stored in the `.raikiri/components` folder. This "assembly caching" strategy leads to near zero cold start for applications.

With Raikiri, you can deploy your own WASM-based FaaS. It facilitates the deploy of web applications by exposing a route for uploading your applications, in a way that you can push to prod with one command.

You can also develop domain-specific applications with Raikiri and integrate them via HTTP by calling the `raikiri.components` domain. Components called by this domain avoid network roundtrips, leading to lower latencies.

Raikiri has support for WASI 0.2.


## Getting started

First install Raikiri:

```sh
cargo install --path .
```

## Server mode

Raikiri includes a local server mode.

To start the server mode:

```sh
raikiri server start --port 3000
```

Clone an example project and create a component:

```sh
git clone https://github.com/viniciusfonseca/raikiri-hello-world.git
cd raikiri-hello-world
raikiri component add --name helloworld --path ./helloworld.wasm
```

Run the component:

```sh
raikiri component run --request '{"username_component_name": "<user>.helloworld","method": "GET","headers": {},"body": ""}''
```