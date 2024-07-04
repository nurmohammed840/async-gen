use async_gen::{gen, GeneratorState};
use futures_core::Stream;
use futures_util::stream::StreamExt;
use std::pin::pin;

#[tokio::test]
async fn noop_stream() {
    let mut gen = pin!(gen! {});
    assert_eq!(gen.resume().await, GeneratorState::Complete(()));
}

#[tokio::test]
async fn empty_stream() {
    let mut ran = false;
    {
        let r = &mut ran;
        let mut gen = pin!(gen! {
            *r = true;
            println!("hello world!");
        });
        assert_eq!(gen.resume().await, GeneratorState::Complete(()));
    }
    assert!(ran);
}

#[tokio::test]
async fn yield_single_value() {
    let mut s = pin!(gen! {
        yield "hello";
    });
    assert_eq!(s.resume().await, GeneratorState::Yielded("hello"));
    assert_eq!(s.resume().await, GeneratorState::Complete(()));
}

#[tokio::test]
async fn yield_multi_value() {
    let mut s = pin!(gen! {
        yield "hello";
        yield "world";
        yield "dizzy";
    });
    assert_eq!(s.resume().await, GeneratorState::Yielded("hello"));
    assert_eq!(s.resume().await, GeneratorState::Yielded("world"));
    assert_eq!(s.resume().await, GeneratorState::Yielded("dizzy"));
    assert_eq!(s.resume().await, GeneratorState::Complete(()));
}

#[tokio::test]
async fn return_stream() {
    fn build_stream() -> impl Stream<Item = i32> {
        gen! {
            yield 1;
            yield 2;
            yield 3;
        }
    }
    let s = build_stream();

    let values: Vec<_> = s.collect().await;
    assert_eq!(3, values.len());
    assert_eq!(1, values[0]);
    assert_eq!(2, values[1]);
    assert_eq!(3, values[2]);
}

#[tokio::test]
async fn consume_channel() {
    let (tx, mut rx) = tokio::sync::mpsc::channel(10);
    let mut s = pin!(gen! {
        while let Some(v) = rx.recv().await {
            yield v;
        }
    });
    for i in 0..3 {
        assert!(tx.send(i).await.is_ok());
        assert_eq!(Some(i), s.next().await);
    }
    drop(tx);
    assert_eq!(None, s.next().await);
}

#[tokio::test]
async fn borrow_self() {
    struct Data(String);

    impl Data {
        fn stream<'a>(&'a self) -> impl Stream<Item = &str> + 'a {
            gen! {
                yield &self.0[..];
            }
        }
    }

    let data = Data("hello".to_string());
    let mut s = pin!(data.stream());
    assert_eq!(Some("hello"), s.next().await);
}

#[tokio::test]
async fn stream_in_stream() {
    let s = gen! {
        let mut s = pin!(gen! {
            for i in 0..3 {
                yield i;
            }
        });
        while let Some(v) = s.next().await {
            yield v;
        }
    };
    let values: Vec<_> = s.collect().await;
    assert_eq!(3, values.len());
}

#[tokio::test]
async fn yield_non_unpin_value() {
    let s: Vec<_> = gen! {
        for i in 0..3 {
            yield async move { i };
        }
    }
    .buffered(1)
    .collect()
    .await;

    assert_eq!(s, vec![0, 1, 2]);
}

#[tokio::test]
async fn unit_yield_in_select() {
    async fn do_stuff_async() {}

    let s = gen! {
        tokio::select! {
            _ = do_stuff_async() => { yield },
            else => { yield },
        };
    };
    let values: Vec<_> = s.collect().await;
    assert_eq!(values.len(), 1);
}

#[tokio::test]
async fn yield_with_select() {
    async fn do_stuff_async() {}
    async fn more_async_work() {}

    let s = gen! {
        tokio::select! {
            _ = do_stuff_async() => { yield "hey" },
            _ = more_async_work() => { yield "hey" },
            else => { yield "hey" },
        };
    };
    let values: Vec<_> = s.collect().await;
    assert_eq!(values, vec!["hey"]);
}

// #[test]
// fn inner_try_stream() {
//     use async_stream::try_stream;
//     use tokio::select;

//     async fn do_stuff_async() {}

//     let _ = stream! {
//         select! {
//             _ = do_stuff_async() => {
//                 let another_s = try_stream! {
//                     yield;
//                 };
//                 let _: Result<(), ()> = Box::pin(another_s).next().await.unwrap();
//             },
//             else => {},
//         }
//         yield
//     };
// }
