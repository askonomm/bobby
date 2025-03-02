# Bobby

A minimal web framework for Rust.

**Note:** very much pre-alpha.

## Installation

Add Bobby to your `Cargo.toml`:

```toml
[dependencies]
bobby = "0.1.1"
```

## Usage

Basic usage looks like this:

```rust
use bobby::{Bobby, Response};

// Create app
let mut app = Bobby::new();

// Config address
app.with_address([127, 0, 0, 1], 3333);

// Routes
app.get("/", |_| Response::html("Hello, <strong>World</strong>"));
app.get("/some/{thing?}", |_| Response::html("An optional route part."));
app.get("/other/{thing}", |_| Response::html("A non-optional route part."));

// Run
app.run();
```

### App configuration

Bobby can be configured using `with_` methods.

#### Address and port

To have Bobby listen on a given address and port, use the `with_address` method:

```rust
app.with_address([127, 0, 0, 1], 3333);
```

If you don't configure this then Bobby will listen on address `127.0.0.1` and port `8080` by default.

#### Logging

Bobby has built-in support for logging with the [log](https://crates.io/crates/log) interface, so you could use any logging library that supports it to generate logs.

An example with using the [env_logger](https://crates.io/crates/env_logger) library:

```rust
let mut logger = env_logger::Builder::from_default_env();
logger.target(env_logger::Target::Stdout);
logger.init();

let mut app = Bobby::new();

// and so forth ...
```

Would then generate the following output in your `stdout`:

```stdout
[2025-03-02T14:05:51Z INFO  bobby::bobby] Listening on 127.0.0.1:3112 ...
[2025-03-02T14:05:56Z INFO  bobby::bobby] HTTP/1.1 GET /
[2025-03-02T14:05:57Z INFO  bobby::bobby] HTTP/1.1 GET /
[2025-03-02T14:05:57Z INFO  bobby::bobby] HTTP/1.1 GET /
[2025-03-02T14:06:01Z INFO  bobby::bobby] HTTP/1.1 GET /asd
[2025-03-02T14:06:01Z WARN  bobby::bobby] HTTP/1.1 GET /asd - Not found
```

### Routing

Routes are added to the instance of `Bobby` by calling route related methods. An example route looks like this:

```rust
app.get("/", |req| {
  Response::html("Hello, World.")
});
```

Supported methods are:

- `get`
- `post`
- `put`
- `delete`
- `patch`
- `options`
- `head`

### Requests

Each route function gets a `Request` instance passed to it as its single argument. 

#### Method

You can see the incoming request' method:

```rust
app.get("/", |req| {
  let method = req.method();
});
```

#### URI

You can see the incoming request' URI:

```rust
app.get("/", |req| {
  let uri = req.uri();
});
```

#### Parameters

You can get the route parameters:

```rust
app.get("/hello/{who}", |req| {
  let who = req.param("who");
});
```

### Responses

Each route must return an instance of `Response`.

#### Response: `HTML`

You can return a HTML response:

```rust
app.get("/", |req| {
  Response::html("Hello, World.")
});
```

#### Response: `JSON`

You can return a JSON response:

```rust
use serde_json::json;

app.get("/", |req| {
  Response::json(json!({
    "name": "John"
  }))
});
```

#### Setting headers

You can set the response headers:

```rust
app.get("/", |req| {
  Response::html("Hello, World.")
    .with_header("Content-Type", "text/html")
});
```

#### Setting status code

You can set the response status:

```rust
app.get("/", |req| {
  Response::html("Not found.")
    .with_header("Content-Type", "text/html")
    .with_status(404)
});
```
