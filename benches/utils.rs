use std::pin::pin;
use std::{future::Future, task::*};

#[inline]
pub fn poll_once<T>(fut: impl Future<Output = T>) -> T {
    let mut cx = const { Context::from_waker(Waker::noop()) };
    let fut = pin!(fut);
    match fut.poll(&mut cx) {
        Poll::Ready(val) => val,
        Poll::Pending => panic!("pending"),
    }
}
