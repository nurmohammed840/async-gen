use std::pin::pin;
use std::{future::Future, task::*};

static DATA: () = ();
static VTABLE: RawWakerVTable = RawWakerVTable::new(|_| raw_waker(), no_op, no_op, no_op);
fn no_op(_: *const ()) {}
fn raw_waker() -> RawWaker {
    RawWaker::new(&DATA, &VTABLE)
}

#[inline]
pub fn poll_once<T>(fut: impl Future<Output = T>) -> T {
    let waker: Waker = unsafe { Waker::from_raw(raw_waker()) };
    let mut cx = Context::from_waker(&waker);
    let fut = pin!(fut);
    match fut.poll(&mut cx) {
        Poll::Ready(val) => val,
        Poll::Pending => panic!("pending"),
    }
}
