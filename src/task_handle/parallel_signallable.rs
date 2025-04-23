use std::future::Future;
use std::pin::{Pin, pin};
use std::task::{Context, Poll};

use futures::future::FusedFuture;
use tokio::sync::oneshot::Sender;
use tokio::task::{JoinHandle, JoinError};

use super::TaskHandle;

pub enum ParallelSignallableTaskHandle <T>
{
	Handle
	{
		handle: JoinHandle <T>,
		shutdown_trigger: Option <Sender <()>>
	},
	Finished
}

impl <T> ParallelSignallableTaskHandle <T>
{
	pub fn new <F> (future: F, shutdown_trigger: Sender <()>) -> Self
	where
		F: Future <Output = T> + Send + 'static,
		T: Send + 'static
	{
		Self::Handle
		{
			handle: tokio::task::spawn (future),
			shutdown_trigger: Some (shutdown_trigger)
		}
	}

	fn unwrap_output_result (output_result: Result <T, JoinError>) -> T
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

				panic! ("task was cancelled")
			}
		}
	}
}

impl <T> TaskHandle for ParallelSignallableTaskHandle <T>
where Self: Future
{
	fn abort (self: Pin <&mut Self>)
	{
		if let Self::Handle {shutdown_trigger, ..} = self . get_mut ()
		{
			if let Some (shutdown_trigger) = shutdown_trigger . take ()
			{
				let _ = shutdown_trigger . send (());
			}
		}
	}
}

impl <T> Future for ParallelSignallableTaskHandle <T>
{
	type Output = T;

	fn poll (mut self: Pin <&mut Self>, cx: &mut Context)
	-> Poll <<Self as Future>::Output>
	{
		match self . as_mut () . get_mut ()
		{
			Self::Handle {handle, ..} => match pin! (handle) . poll (cx)
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

impl <T> FusedFuture for ParallelSignallableTaskHandle <T>
where Self: Future
{
	fn is_terminated (&self) -> bool
	{
		match self
		{
			Self::Handle {..} => false,
			Self::Finished => true
		}
	}
}
