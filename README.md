# Raikiri

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

Create a component:

```sh
raikiri component add --name helloworld --path ./helloworld.wasm
```

Run the component:

```sh
raikiri component run --request '{"username_component_name": "<user>.helloworld","method": "GET","headers": {},"body": ""}''
```