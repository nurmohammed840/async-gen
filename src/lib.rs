#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

mod async_coroutine;
mod async_iter;
mod types;

pub use futures_core;

pub use async_coroutine::{gen, AsyncGen, Yielder};
pub use async_iter::{async_iter_from, AsyncIter};
pub use types::{AsyncGenerator, GeneratorState};

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
/// # #[nio::main]
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

/// Asynchronous stream
#[macro_export]
macro_rules! stream {
    ($($tt:tt)*) => {
        $crate::__private::gen_inner!(($crate) $($tt)*).into_async_iter()
    }
}

#[doc(hidden)]
pub mod __private {
    pub use async_gen_macros::gen_inner;
}
