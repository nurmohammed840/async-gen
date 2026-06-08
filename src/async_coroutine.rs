use std::{
    cell::UnsafeCell,
    future::{poll_fn, Future},
    pin::Pin,
    ptr::NonNull,
    task::{Context, Poll},
};

use crate::{AsyncGenerator, AsyncIter, GeneratorState};

/// Creates a new generator, which implements the [`AsyncGenerator`] trait.
///
/// Also see [`crate::gen!`] macro for more details.
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
pub fn gen<Fut, Y, R>(fut: impl FnOnce(Yielder<Y>) -> Fut) -> AsyncGen<Fut, Y>
where
    Fut: Future<Output = Return<R>>,
{
    let cell = Box::new(UnsafeCell::new(None));
    let data = Box::leak(cell).into();
    let fut = fut(Yielder {
        cell: Cell { data },
    });
    AsyncGen {
        cell: Cell { data },
        fut,
    }
}

struct Cell<Y> {
    data: NonNull<UnsafeCell<Option<Y>>>,
}

impl<Y> Cell<Y> {
    #[inline]
    fn data(&self) -> &UnsafeCell<Option<Y>> {
        unsafe { self.data.as_ref() }
    }
}

unsafe impl<Y: Send> Send for Cell<Y> {}
unsafe impl<Y: Send + Sync> Sync for Cell<Y> {}

/// The return value produced by an async coroutine.
///
/// This wrapper insured that `Yielder` is owned by the `async` body of `gen` function.
/// [`Yielder::return_`] is used to create this value.
#[repr(transparent)]
pub struct Return<T = ()>(T);

#[doc(hidden)]
pub struct Yielder<Y = ()> {
    cell: Cell<Y>,
}

impl<Y> Yielder<Y> {
    /// Same as `yield` keyword.
    ///
    /// It pauses execution and the value is returned to the generator's caller.
    pub fn yield_(&mut self, val: Y) -> impl Future<Output = ()> + use<'_, Y> {
        // SEAFTY: this function is marked with `&mut self`
        //
        // And `Yield<()>` can't escape from this closure:
        //
        // gen(|y: Yield<()>| async {
        //     // `y` can't escape from this closure. and owned by `async` body
        //     y.return_(())
        // });
        unsafe {
            *self.cell.data().get() = Some(val);
        }

        poll_fn(|_| {
            if unsafe { (*self.cell.data().get()).is_some() } {
                return Poll::Pending;
            }
            Poll::Ready(())
        })
    }

    #[inline]
    pub fn return_<R>(self, v: R) -> Return<R> {
        Return(v)
    }
}

/// Represent an asyncronus generator. It implementations [`AsyncGenerator`] trait.
///
/// This `struct` is created by [`gen()`]. See its documentation for more details.
pub struct AsyncGen<Fut, Y> {
    cell: Cell<Y>,
    fut: Fut,
}

impl<Fut, Y> Drop for AsyncGen<Fut, Y> {
    fn drop(&mut self) {
        unsafe {
            drop(Box::from_raw(self.cell.data.as_ptr()));
        };
    }
}

impl<Fut, Y, R> AsyncGen<Fut, Y>
where
    Fut: Future<Output = Return<R>>,
{
    /// See [`AsyncGenerator::poll_resume`] for more details.
    pub fn poll_resume(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<GeneratorState<Y, R>> {
        let me = unsafe { self.get_unchecked_mut() };
        match unsafe { Pin::new_unchecked(&mut me.fut).poll(cx) } {
            Poll::Ready(Return(val)) => Poll::Ready(GeneratorState::Complete(val)),
            Poll::Pending => {
                // SEAFTY: We just return from `me.fut`,
                // So this is safe and unique access to `me.inner.data`
                unsafe {
                    if let Some(val) = (*me.cell.data().get()).take() {
                        return Poll::Ready(GeneratorState::Yielded(val));
                    }
                }
                Poll::Pending
            }
        }
    }

    /// See [`AsyncGenerator::poll_resume`] for more details.
    pub async fn resume(self: &mut Pin<&mut Self>) -> GeneratorState<Y, R> {
        poll_fn(|cx| self.as_mut().poll_resume(cx)).await
    }
}

impl<Fut, Y> AsyncGen<Fut, Y>
where
    Fut: Future<Output = Return<()>>,
{
    /// Creates an async iterator from this generator.
    ///
    /// See [`AsyncIter`] for more details.
    pub fn into_async_iter(self) -> AsyncIter<Self> {
        AsyncIter::from(self)
    }

    /// See [`futures_core::Stream::poll_next`] for more details.
    pub fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Y>> {
        let me = unsafe { self.get_unchecked_mut() };
        match unsafe { Pin::new_unchecked(&mut me.fut).poll(cx) } {
            Poll::Ready(Return(())) => Poll::Ready(None),
            Poll::Pending => {
                // SEAFTY: We just return from `me.fut`,
                // So this is safe and unique access to `me.inner.data`
                unsafe {
                    if let Some(val) = (*me.cell.data().get()).take() {
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
