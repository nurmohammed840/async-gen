This library provides a way to create asynchronous generator using the `async/await` feature in stable Rust.

It is similar to [async-stream](https://docs.rs/async-stream/latest/async_stream/),
But closely mimics the [Coroutine API](https://doc.rust-lang.org/std/ops/trait.Coroutine.html),
Allowing the generator also return a value upon completion, in addition to yielding intermediate values.

# Installation

Add it as a dependency to your Rust project by adding the following line to your `Cargo.toml` file:


```toml
[dependencies]
async-gen = "0.3"
```

# Examples

```rust
use std::pin::pin;
use async_gen::{gen, GeneratorState};

#[nio::main]
async fn main() {
    let g = gen! {
        yield 42;
        return "42"
    };
    let mut g = pin!(g);
    assert_eq!(g.resume().await, GeneratorState::Yielded(42));
    assert_eq!(g.resume().await, GeneratorState::Complete("42"));
}
```
