This library provides a way to create asynchronous generator using the `async/await` feature in stable Rust.

# Installation

Add it as a dependency to your Rust project by adding the following line to your `Cargo.toml` file:


```toml
[dependencies]
async-gen = "0.1"
```

# Examples

```rust
use std::pin::pin;
use async_gen::{gen, GeneratorState};

#[tokio::main]
async fn main() {
    let g = gen! {
        yield 42;
        return "foo"
    };
    let mut g = pin!(g);
    assert_eq!(g.resume().await, GeneratorState::Yielded(42));
    assert_eq!(g.resume().await, GeneratorState::Complete("foo"));
}
```