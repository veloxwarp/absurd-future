//! A future adapter that turns a future that never resolves (i.e., returns `Infallible`)
//! into a future that can resolve to any type.
//!
//! This is useful in scenarios where you have a task that runs forever (like a background
//! service) but need to integrate it into an API that expects a specific return type,
//! such as `tokio::task::JoinSet`.
//!
//! The core of this crate is the [`AbsurdFuture`] struct and the convenient
//! [`absurd_future`] function.
//!
//! For a detailed explanation of the motivation behind this crate and the concept of
//! uninhabited types in Rust async code, see the blog post:
//! [How to use Rust's never type (!) to write cleaner async code](https://academy.fpblock.com/blog/rust-never-type-async-code).
//!
//! # Example
//!
//! ```
//! use std::convert::Infallible;
//! use std::future;
//! use absurd_future::absurd_future;
//!
//! // A future that never completes.
//! async fn task_that_never_returns() -> Infallible {
//!     loop {
//!         // In a real scenario, this might be `tokio::time::sleep` or another
//!         // future that never resolves. For this example, we'll just pend forever.
//!         future::pending::<()>().await;
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     // We have a task that never returns, but we want to use it in a
//!     // context that expects a `Result<(), &str>`.
//!     let future = task_that_never_returns();
//!
//!     // Wrap it with `absurd_future` to change its output type.
//!     let adapted_future: _ = absurd_future::<_, Result<(), &str>>(future);
//!
//!     // This adapted future will now pend forever, just like the original,
//!     // but its type signature satisfies the requirement.
//! }
//! ```

use std::{
    convert::Infallible,
    future::Future,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

/// Turn a never-returning future into a future yielding any desired type.
///
/// This struct is created by the [`absurd_future`] function.
///
/// Useful for async tasks that logically don't complete but need to satisfy an
/// interface expecting a concrete output type. Because the inner future never
/// resolves, this future will also never resolve, so the output type `T` is
/// never actually produced.
#[must_use = "futures do nothing unless polled"]
pub struct AbsurdFuture<F, T> {
    inner: Pin<Box<F>>,
    _marker: PhantomData<fn() -> T>,
}

impl<F, T> AbsurdFuture<F, T> {
    /// Creates a new `AbsurdFuture` that wraps the given future.
    ///
    /// The inner future must have an output type of `Infallible`.
    pub fn new(inner: F) -> Self {
        Self {
            inner: Box::pin(inner),
            _marker: PhantomData,
        }
    }
}

impl<F, T> Future for AbsurdFuture<F, T>
where
    F: Future<Output = Infallible>,
{
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let inner = self.get_mut().inner.as_mut();
        match Future::poll(inner, cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(never) => match never {},
        }
    }
}

/// Wraps a future that never returns and gives it an arbitrary output type.
///
/// This function makes it easier to create an [`AbsurdFuture`].
///
/// # Type Parameters
///
/// - `F`: The type of the inner future, which must return `Infallible`.
/// - `T`: The desired output type for the wrapped future. This is often inferred.
pub fn absurd_future<F, T>(future: F) -> AbsurdFuture<F, T>
where
    F: Future<Output = Infallible>,
{
    AbsurdFuture::new(future)
}

#[cfg(test)]
mod tests {
    use super::absurd_future;
    use std::{
        convert::Infallible,
        future::{self, Future},
        task::Poll,
    };

    #[tokio::test]
    async fn adapted_pending_future_remains_pending() {
        let adapted = absurd_future::<_, String>(future::pending::<Infallible>());
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(10), adapted)
                .await
                .is_err()
        );
    }

    #[test]
    fn adapted_future_can_be_polled_without_unpinning_inner_future() {
        let adapted = absurd_future::<_, ()>(future::pending::<Infallible>());
        let mut adapted = Box::pin(adapted);
        let waker = std::task::Waker::noop();
        let mut context = std::task::Context::from_waker(waker);
        assert!(matches!(adapted.as_mut().poll(&mut context), Poll::Pending));
    }
}
