use async_gen::{gen, GeneratorState};
use futures::executor::block_on;
use futures_core::Stream;
use futures_util::stream::StreamExt;
use std::pin::pin;

#[test]
fn noop_stream() {
    block_on(async {
        let mut gen = pin!(gen! {});
        assert_eq!(gen.resume().await, GeneratorState::Complete(()));
    })
}

#[test]
fn empty_stream() {
    block_on(async {
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
    });
}

#[test]
fn yield_single_value() {
    block_on(async {
        let mut s = pin!(gen! {
            yield "hello";
        });
        assert_eq!(s.resume().await, GeneratorState::Yielded("hello"));
        assert_eq!(s.resume().await, GeneratorState::Complete(()));
    })
}

#[test]
fn fused() {
    block_on(async {
        let s = pin!(gen! {
            yield "hello";
        });
        let mut s = s.fuse();
        assert_eq!(s.next().await, Some("hello"));
        assert_eq!(s.next().await, None);
        assert_eq!(s.next().await, None);
    });
}

#[test]
fn yield_multi_value() {
    block_on(async {
        let mut s = pin!(gen! {
            yield "hello";
            yield "world";
            yield "dizzy";
        });
        assert_eq!(s.resume().await, GeneratorState::Yielded("hello"));
        assert_eq!(s.resume().await, GeneratorState::Yielded("world"));
        assert_eq!(s.resume().await, GeneratorState::Yielded("dizzy"));
        assert_eq!(s.resume().await, GeneratorState::Complete(()));
    })
}

#[test]
fn return_stream() {
    block_on(async {
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
    })
}

#[test]
fn consume_channel() {
    block_on(async {
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
    });
}

#[test]
fn borrow_self() {
    block_on(async {
        struct Data(String);

        impl Data {
            fn stream(&self) -> impl Stream<Item = &str> + '_ {
                gen! {
                    yield &self.0[..];
                }
            }
        }

        let data = Data("hello".to_string());
        let mut s = pin!(data.stream());
        assert_eq!(Some("hello"), s.next().await);
    })
}

#[test]
fn stream_in_stream() {
    block_on(async {
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
    })
}

#[test]
fn yield_non_unpin_value() {
    block_on(async {
        let s: Vec<_> = gen! {
            for i in 0..3 {
                yield async move { i };
            }
        }
        .buffered(1)
        .collect()
        .await;

        assert_eq!(s, vec![0, 1, 2]);
    })
}

#[test]
fn unit_yield_in_select() {
    block_on(async {
        async fn do_stuff_async() {}

        let s = gen! {
            tokio::select! {
                _ = do_stuff_async() => { yield },
                else => { yield },
            };
        };
        let values: Vec<_> = s.collect().await;
        assert_eq!(values.len(), 1);
    })
}

#[test]
fn yield_with_select() {
    block_on(async {
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
    })
}
