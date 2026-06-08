use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures_core::Stream;
use pin_project_lite::pin_project;

use crate::{AsyncGenerator, GeneratorState};

pin_project! {
    /// An async iterator over the values yielded by an underlying generator.
    ///
    /// ## Example
    ///
    /// ```
    /// use async_gen::{gen, AsyncIter};
    /// use futures_core::Stream;
    /// use futures_util::StreamExt;
    ///
    /// fn get_async_iter() -> impl Stream<Item = i32> {
    ///     AsyncIter::from(gen! {
    ///         yield 1;
    ///         yield 2;
    ///         yield 3;
    ///     })
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let it = get_async_iter();
    ///     let v: Vec<_> = it.collect().await;
    ///     assert_eq!(v, [1, 2, 3]);
    /// }
    /// ```
    #[derive(Clone)]
    pub struct AsyncIter<G> {
        #[pin]
        gen: G,
    }
}

impl<G> AsyncIter<G>
where
    G: AsyncGenerator<Return = ()>,
{
    /// Attempt to pull out the next value of this async iterator, registering the
    /// current task for wakeup if the value is not yet available, and returning
    /// `None` if the async iterator is exhausted.
    ///
    /// # Return value
    ///
    /// There are several possible return values, each indicating a distinct
    /// async iterator state:
    ///
    /// - `Poll::Pending` means that this async iterator's next value is not ready
    ///   yet. Implementations will ensure that the current task will be notified
    ///   when the next value may be ready.
    ///
    /// - `Poll::Ready(Some(val))` means that the async iterator has successfully
    ///   produced a value, `val`, and may produce further values on subsequent
    ///   `poll_next` calls.
    ///
    /// - `Poll::Ready(None)` means that the async iterator has terminated, and
    ///   `poll_next` should not be invoked again.
    ///
    /// # Panics
    ///
    /// Once an async iterator has finished (returned `Ready(None)` from `poll_next`), calling its
    /// `poll_next` method again may panic, block forever, or cause other kinds of
    /// problems; the `AsyncIterator` trait places no requirements on the effects of
    /// such a call. However, as the `poll_next` method is not marked `unsafe`,
    /// Rust's usual rules apply: calls must never cause undefined behavior
    /// (memory corruption, incorrect use of `unsafe` functions, or the like),
    /// regardless of the async iterator's state.
    #[inline]
    pub fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<G::Yield>> {
        self.project().gen.poll_resume(cx).map(|s| match s {
            GeneratorState::Yielded(val) => Some(val),
            GeneratorState::Complete(()) => None,
        })
    }
}

impl<G> Stream for AsyncIter<G>
where
    G: AsyncGenerator<Return = ()>,
{
    type Item = G::Yield;

    #[inline]
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        AsyncIter::poll_next(self, cx)
    }
}

impl<S: Stream> AsyncGenerator for S {
    type Yield = S::Item;
    type Return = ();

    #[inline]
    fn poll_resume(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<GeneratorState<Self::Yield, Self::Return>> {
        self.poll_next(cx).map(|val| match val {
            Some(val) => GeneratorState::Yielded(val),
            None => GeneratorState::Complete(()),
        })
    }
}

/// Converts an [`AsyncGenerator`] into an async iterator.
#[inline]
pub fn async_iter_from<G>(gen: G) -> impl Stream<Item = G::Yield>
where
    G: AsyncGenerator<Return = ()>,
{
    AsyncIter { gen }
}

impl<G> From<G> for AsyncIter<G>
where
    G: AsyncGenerator<Return = ()>,
{
    #[inline]
    fn from(gen: G) -> Self {
        AsyncIter { gen }
    }
}
