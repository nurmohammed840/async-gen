#![doc = include_str!("../README.md")]

pub use futures_core;
use pin_project_lite::pin_project;
use std::{
    cell::UnsafeCell,
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
pub enum GeneratorState<Y, R> {
    Yielded(Y),
    Complete(R),
}

pub trait AsyncGenerator {
    type Yield;
    type Return;

    fn resume(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<GeneratorState<Self::Yield, Self::Return>>;
}

#[derive(Debug)]
struct Inner<Y> {
    data: UnsafeCell<Option<Y>>,
}

unsafe impl<Y: Send> Send for Inner<Y> {}
unsafe impl<Y: Sync> Sync for Inner<Y> {}

pub struct Yield<Y> {
    inner: Arc<Inner<Y>>,
}

impl<Y> Yield<Y> {
    pub fn yield_(&mut self, val: Y) -> impl Future + '_ {
        *unsafe { &mut *self.inner.data.get() } = Some(val);
        std::future::poll_fn(|_| {
            if unsafe { &*self.inner.data.get() }.is_some() {
                return Poll::Pending;
            }
            Poll::Ready(())
        })
    }
}

pin_project! {
    #[derive(Debug)]
    pub struct AsyncGen<Y, Fut> {
        inner: Arc<Inner<Y>>,
        #[pin]
        fut: Fut,
    }
}

impl<Y, R, Fut> AsyncGen<Y, Fut>
where
    Fut: Future<Output = (Yield<Y>, R)>,
{
    pub fn new(fut: impl FnOnce(Yield<Y>) -> Fut) -> Self {
        let inner = Arc::new(Inner {
            data: UnsafeCell::new(None),
        });
        let fut = fut(Yield {
            inner: inner.clone(),
        });
        Self { inner, fut }
    }

    pub fn poll_resume(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<GeneratorState<Y, R>> {
        let me = self.project();
        match me.fut.poll(cx) {
            Poll::Ready((_, val)) => Poll::Ready(GeneratorState::Complete(val)),
            Poll::Pending => {
                if let Some(val) = unsafe { &mut *me.inner.data.get() }.take() {
                    return Poll::Ready(GeneratorState::Yielded(val));
                }
                Poll::Pending
            }
        }
    }

    pub async fn resume(self: &mut Pin<&mut Self>) -> GeneratorState<Y, R> {
        std::future::poll_fn(|cx| self.as_mut().poll_resume(cx)).await
    }

    pub fn into_stream(self) -> AsyncStream<Self> {
        AsyncStream::from(self)
    }
}

impl<Y, R, Fut: Future<Output = (Yield<Y>, R)>> AsyncGenerator for AsyncGen<Y, Fut> {
    type Yield = Y;
    type Return = R;

    fn resume(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<GeneratorState<Self::Yield, Self::Return>> {
        self.poll_resume(cx)
    }
}

pin_project! {
    #[derive(Debug)]
    pub struct AsyncStream<G> {
        done: bool,
        #[pin]
        gen: G,
    }
}

impl<G> From<G> for AsyncStream<G> {
    #[inline]
    fn from(gen: G) -> Self {
        AsyncStream { done: false, gen }
    }
}

impl<T, G> futures_core::Stream for AsyncStream<G>
where
    G: AsyncGenerator<Yield = T>,
{
    type Item = T;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let me = self.project();
        if *me.done {
            return Poll::Ready(None);
        }
        me.gen.resume(cx).map(|s| match s {
            GeneratorState::Yielded(val) => Some(val),
            GeneratorState::Complete(_) => {
                *me.done = true;
                None
            }
        })
    }
}

#[macro_export]
macro_rules! gen {
    ($($tt:tt)*) => {
        $crate::__private::gen_inner!(($crate) $($tt)*)
    }
}

#[doc(hidden)]
pub mod __private {
    pub use async_gen_macros::*;
}
