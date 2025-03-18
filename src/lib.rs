//! Transpose Result/Option of Future to Future of Result/Option.
//!
//! For collections of multiple elements use join instead.
//!
//! ```
//! # use transpose_future::TransposeFuture;
//! # async fn m() {
//! let x: Option<i32> = Some(async {5}).transpose().await;
//! # }
//! ```
use std::{
    future::{Future, IntoFuture},
    pin::Pin,
};

pub trait TransposeFuture {
    type Output: Future;
    fn transpose(self) -> Self::Output;
}
impl<F: IntoFuture> TransposeFuture for Option<F> {
    type Output = TransposedOption<F::IntoFuture>;
    /// Transpose an Option<impl Future<Output = T>> to an impl Future<Output = Option<T>>
    ///
    /// ```
    /// # use transpose_future::TransposeFuture;
    /// # async fn m() {
    /// let x: Option<i32> = Some(async {5}).transpose().await;
    /// # }
    /// ```
    fn transpose(self) -> Self::Output {
        TransposedOption(self.map(IntoFuture::into_future))
    }
}
pub struct TransposedOption<F>(Option<F>);
impl<F: Future> Future for TransposedOption<F> {
    type Output = Option<F::Output>;
    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        // SAFETY: We do not move here, just get a reference to the inner value. There is no other data.
        match unsafe { self.map_unchecked_mut(|x| &mut x.0) }.as_pin_mut() {
            Some(f) => f.poll(cx).map(Some),
            None => std::task::Poll::Ready(None),
        }
    }
}
impl<F: IntoFuture, T: Unpin> TransposeFuture for Result<F, T> {
    type Output = TransposedResult<F::IntoFuture, T>;
    /// Transpose an Result<impl Future<Output = T>, E> to an impl Future<Output = Result<T, E>>
    ///
    /// ```
    /// # use transpose_future::TransposeFuture;
    /// # async fn m() {
    /// let x: Result<i32, ()> = Ok(async {5}).transpose().await;
    /// # }
    /// ```
    fn transpose(self) -> Self::Output {
        TransposedResult(self.map(IntoFuture::into_future).map_err(Some))
    }
}
pub struct TransposedResult<F, T>(Result<F, Option<T>>);
impl<F: Future, T: Unpin> Future for TransposedResult<F, T> {
    type Output = Result<F::Output, T>;
    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        // SAFETY: We do not move anything here, we essentially do as_pin_mut on the inner value.
        let mapped = unsafe {
            let x = self.get_unchecked_mut();
            match &mut x.0 {
                Ok(f) => Ok(Pin::new_unchecked(f)),
                Err(e) => Err(Pin::new_unchecked(e)),
            }
        };
        match mapped {
            Ok(f) => f.poll(cx).map(Ok),
            Err(e) => std::task::Poll::Ready(Err(e.get_mut().take().unwrap())),
        }
    }
}
