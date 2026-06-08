use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures_core::Stream;

use crate::{AsyncGenerator, GeneratorState};

/// An async iterator over the values yielded by an underlying generator.
///
/// ## Example
///
/// ```
/// use async_gen::{gen, AsyncIter};
/// use futures::{StreamExt, Stream};
///
/// fn get_async_iter() -> impl Stream<Item = i32> {
///     AsyncIter::from(gen! {
///         yield 1;
///         yield 2;
///         yield 3;
///     })
/// }
///
/// #[nio::main]
/// async fn main() {
///     let it = get_async_iter();
///     let v: Vec<_> = it.collect().await;
///     assert_eq!(v, [1, 2, 3]);
/// }
/// ```
#[derive(Clone)]
pub struct AsyncIter<G> {
    gen: G,
}

impl<G> AsyncIter<G>
where
    G: AsyncGenerator<Return = ()>,
{
    /// See [`Stream::poll_next`] for more details.
    #[inline]
    pub fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<G::Yield>> {
        unsafe {
            let me = self.get_unchecked_mut();
            Pin::new_unchecked(&mut me.gen).poll_resume(cx)
        }
        .map(|s| match s {
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

/// Converts an [`AsyncGenerator`] into an async iterator.
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
    fn from(gen: G) -> Self {
        AsyncIter { gen }
    }
}
