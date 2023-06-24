#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

pub use futures_core;
use pin_project_lite::pin_project;
use std::{
    cell::UnsafeCell,
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

/// The result of a generator resumption.
///
/// This enum is returned from the `Generator::resume` method and indicates the
/// possible return values of a generator. Currently this corresponds to either
/// a suspension point (`Yielded`) or a termination point (`Complete`).
#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
pub enum GeneratorState<Y, R> {
    /// The generator suspended with a value.
    ///
    /// This state indicates that a generator has been suspended, and typically
    /// corresponds to a `yield` statement. The value provided in this variant
    /// corresponds to the expression passed to `yield` and allows generators to
    /// provide a value each time they yield.
    Yielded(Y),

    /// The generator completed with a return value.
    ///
    /// This state indicates that a generator has finished execution with the
    /// provided value. Once a generator has returned `Complete` it is
    /// considered a programmer error to call `resume` again.
    Complete(R),
}

/// Generators, also commonly referred to as coroutines.
pub trait AsyncGenerator {
    /// The type of value this generator yields.
    ///
    /// This associated type corresponds to the `yield` expression and the
    /// values which are allowed to be returned each time a generator yields.
    /// For example an iterator-as-a-generator would likely have this type as
    /// `T`, the type being iterated over.
    type Yield;

    /// The type of value this generator returns.
    ///
    /// This corresponds to the type returned from a generator either with a
    /// `return` statement or implicitly as the last expression of a generator
    /// literal. For example futures would use this as `Result<T, E>` as it
    /// represents a completed future.
    type Return;

    /// Resumes the execution of this generator.
    ///
    /// This function will resume execution of the generator or start execution
    /// if it hasn't already. This call will return back into the generator's
    /// last suspension point, resuming execution from the latest `yield`. The
    /// generator will continue executing until it either yields or returns, at
    /// which point this function will return.
    ///
    /// # Return value
    ///
    /// The `GeneratorState` enum returned from this function indicates what
    /// state the generator is in upon returning. If the `Yielded` variant is
    /// returned then the generator has reached a suspension point and a value
    /// has been yielded out. Generators in this state are available for
    /// resumption at a later point.
    ///
    /// If `Complete` is returned then the generator has completely finished
    /// with the value provided. It is invalid for the generator to be resumed
    /// again.
    ///
    /// # Panics
    ///
    /// This function may panic if it is called after the `Complete` variant has
    /// been returned previously. While generator literals in the language are
    /// guaranteed to panic on resuming after `Complete`, this is not guaranteed
    /// for all implementations of the `Generator` trait.
    fn poll_resume(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<GeneratorState<Self::Yield, Self::Return>>;
}

struct Inner<Y> {
    data: UnsafeCell<Option<Y>>,
}

unsafe impl<Y: Send + Sync> Sync for Inner<Y> {}

#[doc(hidden)]
pub struct Yield<Y = ()> {
    inner: Arc<Inner<Y>>,
}

#[doc(hidden)]
pub struct Return<T = ()>(T);

impl<Y> Yield<Y> {
    /// Same as `yield` keyword.
    ///
    /// It pauses execution and the value is returned to the generator's caller.
    pub async fn yield_(&mut self, val: Y) {
        // SEAFTY: this function is marked with `&mut self`
        //
        // And `Yield<()>` can't escape from this closure:
        //
        // gen(|y: Yield<()>| async {
        //     // `y` can't escape from this closure. and owned by `async` body
        //     y.return_(())
        // });
        unsafe {
            *self.inner.data.get() = Some(val);
        }
        std::future::poll_fn(|_| {
            if unsafe { (*self.inner.data.get()).is_some() } {
                return Poll::Pending;
            }
            Poll::Ready(())
        })
        .await
    }

    #[inline]
    pub fn return_<R>(self, _v: R) -> Return<R> {
        Return(_v)
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
    Fut: Future<Output = Return<R>>,
{
    /// See [`AsyncGenerator::poll_resume`] for more details.
    #[doc(hidden)]
    pub fn poll_resume(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<GeneratorState<Y, R>> {
        let me = self.project();
        match me.fut.poll(cx) {
            Poll::Ready(Return(val)) => Poll::Ready(GeneratorState::Complete(val)),
            Poll::Pending => {
                // SEAFTY: We just return from `me.fut`,
                // So this is safe and unique access to `me.inner.data`
                unsafe {
                    let data = &mut *me.inner.data.get();
                    if data.is_some() {
                        return Poll::Ready(GeneratorState::Yielded(data.take().unwrap()));
                    }
                }
                Poll::Pending
            }
        }
    }

    #[inline]
    /// See [`AsyncGenerator::poll_resume`] for more details.
    pub async fn resume(self: &mut Pin<&mut Self>) -> GeneratorState<Y, R> {
        std::future::poll_fn(|cx| self.as_mut().poll_resume(cx)).await
    }
}

impl<Fut, Y> AsyncGen<Fut, Y>
where
    Fut: Future<Output = Return<()>>,
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
            Poll::Ready(Return(())) => Poll::Ready(None),
            Poll::Pending => {
                // SEAFTY: We just return from `me.fut`,
                // So this is safe and unique access to `me.inner.data`
                unsafe {
                    let data = &mut *me.inner.data.get();
                    if data.is_some() {
                        return Poll::Ready(data.take());
                    }
                }
                Poll::Pending
            }
        }
    }
}

impl<Fut, Y> futures_core::Stream for AsyncGen<Fut, Y>
where
    Fut: Future<Output = Return<()>>,
{
    type Item = Y;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        AsyncGen::poll_next(self, cx)
    }
}

impl<Fut, Y, R> AsyncGenerator for AsyncGen<Fut, Y>
where
    Fut: Future<Output = Return<R>>,
{
    type Yield = Y;
    type Return = R;

    fn poll_resume(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<GeneratorState<Self::Yield, Self::Return>> {
        AsyncGen::poll_resume(self, cx)
    }
}

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

impl<G> From<G> for AsyncIter<G> {
    #[inline]
    fn from(gen: G) -> Self {
        AsyncIter { gen }
    }
}

impl<G: AsyncGenerator<Return = ()>> AsyncIter<G> {
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
    /// yet. Implementations will ensure that the current task will be notified
    /// when the next value may be ready.
    ///
    /// - `Poll::Ready(Some(val))` means that the async iterator has successfully
    /// produced a value, `val`, and may produce further values on subsequent
    /// `poll_next` calls.
    ///
    /// - `Poll::Ready(None)` means that the async iterator has terminated, and
    /// `poll_next` should not be invoked again.
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
    pub fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<G::Yield>> {
        self.project().gen.poll_resume(cx).map(|s| match s {
            GeneratorState::Yielded(val) => Some(val),
            GeneratorState::Complete(()) => None,
        })
    }
}

impl<G: AsyncGenerator<Return = ()>> futures_core::Stream for AsyncIter<G> {
    type Item = G::Yield;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        AsyncIter::poll_next(self, cx)
    }
}

/// Creates a new generator, which implements the [`AsyncGenerator`] trait.
///
/// Also see [`gen!`] macro for more details.
///
/// ## Examples
///
/// ```
/// use async_gen::{gen, AsyncGen, AsyncGenerator, Return};
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
/// fn check_type_1(_: &AsyncGen<impl Future<Output = Return<&'static str>>, i32>) {}
/// fn check_type_2(_: &impl AsyncGenerator<Yield = i32, Return = &'static str>) {}
/// ```
pub fn gen<Fut, Y, R>(fut: impl FnOnce(Yield<Y>) -> Fut) -> AsyncGen<Fut, Y>
where
    Fut: Future<Output = Return<R>>,
{
    let inner = Arc::new(Inner {
        data: UnsafeCell::new(None),
    });
    let fut = fut(Yield {
        inner: inner.clone(),
    });
    AsyncGen { inner, fut }
}

/// A macro for creating generator.
///
/// Also see [`gen()`] function for more details.
///
/// ## Examples
///
/// ```
/// use std::pin::pin;
/// use async_gen::{gen, GeneratorState};
///
/// # #[tokio::main]
/// # async fn main() {
/// let gen = gen! {
///     yield 42;
///     return "42"
/// };
/// let mut g = pin!(gen);
/// assert_eq!(g.resume().await, GeneratorState::Yielded(42));
/// assert_eq!(g.resume().await, GeneratorState::Complete("42"));
/// # }
/// ```
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
