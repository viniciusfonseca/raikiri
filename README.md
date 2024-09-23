# Raikiri

First install Raikiri:

```sh
cargo install raikiri
```

Run a WASM component:

```sh
raikiri wasm run ./helloworld.wasm
```

## Server mode

Raikiri also includes a local server mode.

To start the server mode:

```sh
raikiri server start --port 3000
```

Create a `~/.raikiriconf` file with the following content to use the local server with Raikiri CLI: 

```
URL=http://localhost:3000
```

Create a component:

```sh
raikiri comopnent add --name helloworld --path ./helloworld.wasm
```

Run the component:

```sh
raikiri component run --name helloworld --params '{"param0": "foo", "param1": "bar"}'
```