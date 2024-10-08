use async_gen::{gen, GeneratorState};
use futures::executor::block_on;
use std::pin::pin;

#[test]
fn single_err() {
    block_on(async {
        let mut s = pin!(gen! {
            if true {
                Err("hello")?;
            } else {
                yield "world";
            }
            Result::<_, &str>::Ok(())
        });
        assert_eq!(s.resume().await, GeneratorState::Complete(Err("hello")));
    })
}

#[test]
fn yield_then_err() {
    block_on(async {
        let mut s = pin!(gen! {
            yield "hello";
            Err("world")?;
            Ok(())
        });
        assert_eq!(s.resume().await, GeneratorState::Yielded("hello"));
        assert_eq!(s.resume().await, GeneratorState::Complete(Err("world")));
    })
}
