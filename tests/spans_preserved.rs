use async_gen::gen;
use futures_util::stream::StreamExt;
use std::pin::pin;

#[tokio::test]
async fn spans_preserved() {
    let mut s = pin!(gen! {
     assert_eq!(line!(), 8);
    });

    while s.next().await.is_some() {
        unreachable!();
    }
}
