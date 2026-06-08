#![allow(clippy::never_loop)]

use async_gen::stream;
use futures::StreamExt;
use std::pin::pin;

#[nio::test]
async fn spans_preserved() {
    let mut s = pin!(stream! {
        assert_eq!(line!(), 10);
    });

    while s.next().await.is_some() {
        unreachable!();
    }
}
