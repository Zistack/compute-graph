use std::future::Future;
use std::pin::{Pin, pin};
use std::task::{Context, Poll};

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
where T: Unpin + Default
{
	fn abort (&mut self)
	{
		if let Self::Handle (handle) = self
		{
			handle . abort ();
		}
	}
}

impl <T> Future for ParallelCancellableTaskHandle <T>
where T: Unpin + Default
{
	type Output = T;

	fn poll (self: Pin <&mut Self>, cx: &mut Context)
	-> Poll <<Self as Future>::Output>
	{
		let mut_self = self . get_mut ();

		match std::mem::replace (mut_self, Self::Finished)
		{
			Self::Handle (mut handle) => match pin! (&mut handle) . poll (cx)
			{
				Poll::Pending =>
				{
					*mut_self = Self::Handle (handle);
					Poll::Pending
				},
				Poll::Ready (output_result) =>
					Poll::Ready (Self::unwrap_output_result (output_result))
			}
			Self::Finished =>
				panic! ("task handle was polled after output was taken")
		}
	}
}
