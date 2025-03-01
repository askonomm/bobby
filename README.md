# Bobby

A minimal web framework for Rust.

**Note:** very much pre-alpha.

## Installation

Add Bobby to your `Cargo.toml`:

```toml
[dependencies]
bobby = "0.0.1"
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
app.run().ok();
```

### App configuration

To be written.

### Routing

To be written.

### Requests

To be written.

### Responses

To be written.
