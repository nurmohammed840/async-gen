# Examples

```rust
use async_gen::{gen, GeneratorState};
use std::pin::pin;

#[tokio::main]
async fn main() {
    let gen = gen!(async {
        for i in 0..2 {
            yield i;
        }
        return "Done";
    });
    let mut gen = pin!(gen);
    assert_eq!(gen.resume().await, GeneratorState::Yielded(0));
    assert_eq!(gen.resume().await, GeneratorState::Yielded(1));
    assert_eq!(gen.resume().await, GeneratorState::Complete("Done"));
}
```

Here is the same example without using the `gen!` macro.

```rust
use async_gen::{AsyncGen, AsyncGenerator};

fn without_macro() -> impl AsyncGenerator {
    AsyncGen::new(|mut c| async {
        for i in 0..2 {
            c.yield_(i).await;
        }
        return (c, "Done");
    })
}
```

Here is an example of asynchronous generator that yields numbers from `0` to `9`

```rust
use async_gen::{futures_core::Stream, gen};

fn numbers() -> impl Stream<Item = i32> {
    let gen = gen!(async {
        for i in 0..10 {
            yield i;
        }
    });
    gen.into_stream()
}
```