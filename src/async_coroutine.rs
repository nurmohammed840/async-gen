use std::{
    cell::UnsafeCell,
    future::{poll_fn, Future},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use pin_project_lite::pin_project;

use crate::{AsyncGenerator, AsyncIter, GeneratorState};

/// Creates a new generator, which implements the [`AsyncGenerator`] trait.
///
/// Also see [`gen!`] macro for more details.
///
/// ## Examples
///
/// ```
/// use async_gen::{gen, AsyncGen, AsyncGenerator};
/// use std::future::Future;
///
/// fn example() {
///     let g = gen(|mut c| async {
///         c.yield_(42).await;
///         c.return_("42")
///     });
///
///     check_type_1(&g);
///     check_type_2(&g);
/// }
/// fn check_type_1(_: &AsyncGen<impl Future<Output = &'static str>, i32>) {}
/// fn check_type_2(_: &impl AsyncGenerator<Yield = i32, Return = &'static str>) {}
/// ```
pub fn gen<Fut, Y, R>(fut: impl FnOnce(Yielder<Y>) -> Fut) -> AsyncGen<Fut, Y>
where
    Fut: Future<Output = R>,
{
    let inner = Arc::new(Inner {
        data: UnsafeCell::new(None),
    });
    let fut = fut(Yielder {
        inner: inner.clone(),
    });
    AsyncGen { inner, fut }
}

struct Inner<Y> {
    data: UnsafeCell<Option<Y>>,
}

unsafe impl<Y: Send> Send for Inner<Y> {}
unsafe impl<Y: Send + Sync> Sync for Inner<Y> {}

#[doc(hidden)]
pub struct Yielder<Y = ()> {
    inner: Arc<Inner<Y>>,
}

impl<Y> Yielder<Y> {
    /// Same as `yield` keyword.
    ///
    /// It pauses execution and the value is returned to the generator's caller.
    pub async fn yield_(&mut self, val: Y) {
        // SEAFTY: this function is marked with `&mut self`
        //
        // And `Yield<()>` can't escape from this closure:
        //
        // gen(|y: Yield<()>| async {
        //     // `y` can't escape from this closure. and must owned by `async` body
        // });
        unsafe {
            *self.inner.data.get() = Some(val);
        }

        poll_fn(|_| {
            if unsafe { (*self.inner.data.get()).is_some() } {
                return Poll::Pending;
            }
            Poll::Ready(())
        })
        .await
    }
}

pin_project! {
    /// Represent an asyncronus generator. It implementations [`AsyncGenerator`] trait.
    ///
    /// This `struct` is created by [`gen()`]. See its documentation for more details.
    pub struct AsyncGen<Fut, Y> {
        inner: Arc<Inner<Y>>,
        #[pin]
        fut: Fut,
    }
}

impl<Fut, Y, R> AsyncGen<Fut, Y>
where
    Fut: Future<Output = R>,
{
    /// See [`AsyncGenerator::poll_resume`] for more details.
    #[doc(hidden)]
    pub fn poll_resume(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<GeneratorState<Y, R>> {
        let me = self.project();
        match me.fut.poll(cx) {
            Poll::Ready(val) => Poll::Ready(GeneratorState::Complete(val)),
            Poll::Pending => {
                // SEAFTY: We just return from `me.fut`,
                // So this is safe and unique access to `me.inner.data`
                unsafe {
                    if let Some(val) = (*me.inner.data.get()).take() {
                        return Poll::Ready(GeneratorState::Yielded(val));
                    }
                }
                Poll::Pending
            }
        }
    }

    #[inline]
    /// See [`AsyncGenerator::poll_resume`] for more details.
    pub async fn resume(self: &mut Pin<&mut Self>) -> GeneratorState<Y, R> {
        poll_fn(|cx| self.as_mut().poll_resume(cx)).await
    }
}

impl<Fut, Y> AsyncGen<Fut, Y>
where
    Fut: Future<Output = ()>,
{
    #[inline]
    /// Creates an async iterator from this generator.
    ///
    /// See [`AsyncIter`] for more details.
    pub fn into_async_iter(self) -> AsyncIter<Self> {
        AsyncIter::from(self)
    }

    #[doc(hidden)]
    pub fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Y>> {
        let me = self.project();
        match me.fut.poll(cx) {
            Poll::Ready(()) => Poll::Ready(None),
            Poll::Pending => {
                // SEAFTY: We just return from `me.fut`,
                // So this is safe and unique access to `me.inner.data`
                unsafe {
                    if let Some(val) = (*me.inner.data.get()).take() {
                        return Poll::Ready(Some(val));
                    }
                }
                Poll::Pending
            }
        }
    }
}

impl<Fut, Y, R> AsyncGenerator for AsyncGen<Fut, Y>
where
    Fut: Future<Output = R>,
{
    type Yield = Y;
    type Return = R;

    #[inline]
    fn poll_resume(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<GeneratorState<Self::Yield, Self::Return>> {
        AsyncGen::poll_resume(self, cx)
    }
}
