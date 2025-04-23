use std::future::Future;
use std::pin::{Pin, pin};
use std::task::{Context, Poll};

use futures::future::FusedFuture;
use tokio::task::{JoinHandle, JoinError};

use super::TaskHandle;

pub enum ParallelCancellableTaskHandle <T>
{
	Handle (JoinHandle <T>),
	Finished
}

impl <T> ParallelCancellableTaskHandle <T>
{
	pub fn new <F> (future: F) -> Self
	where
		F: Future <Output = T> + Send + 'static,
		T: Send + 'static
	{
		Self::Handle (tokio::task::spawn (future))
	}

	fn unwrap_output_result (output_result: Result <T, JoinError>) -> T
	where T: Default
	{
		match output_result
		{
			Ok (output) => output,
			Err (join_error) =>
			{
				if let Ok (panic) = join_error . try_into_panic ()
				{
					std::panic::resume_unwind (panic);
				}

				T::default ()
			}
		}
	}
}

impl <T> TaskHandle for ParallelCancellableTaskHandle <T>
where Self: Future
{
	fn abort (self: Pin <&mut Self>)
	{
		if let Self::Handle (handle) = self . get_mut ()
		{
			handle . abort ();
		}
	}
}

impl <T> Future for ParallelCancellableTaskHandle <T>
where T: Default
{
	type Output = T;

	fn poll (mut self: Pin <&mut Self>, cx: &mut Context)
	-> Poll <<Self as Future>::Output>
	{
		match self . as_mut () . get_mut ()
		{
			Self::Handle (handle) => match pin! (handle) . poll (cx)
			{
				Poll::Pending => Poll::Pending,
				Poll::Ready (output_result) =>
				{
					self . set (Self::Finished);
					Poll::Ready (Self::unwrap_output_result (output_result))
				}
			},
			Self::Finished =>
				panic! ("task handle was polled after output was taken")
		}
	}
}

impl <T> FusedFuture for ParallelCancellableTaskHandle <T>
where Self: Future
{
	fn is_terminated (&self) -> bool
	{
		match self
		{
			Self::Handle (_) => false,
			Self::Finished => true
		}
	}
}
