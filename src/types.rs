use std::{
    pin::Pin,
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
