use std::future::Future;
use std::pin::{Pin, pin};
use std::task::{Context, Poll};

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
where T: Unpin
{
	fn abort (&mut self)
	{
		if let Self::Handle {shutdown_trigger, ..} = self
		{
			if let Some (shutdown_trigger) = shutdown_trigger . take ()
			{
				let _ = shutdown_trigger . send (());
			}
		}
	}
}

impl <T> Future for ParallelSignallableTaskHandle <T>
where T: Unpin
{
	type Output = T;

	fn poll (self: Pin <&mut Self>, cx: &mut Context)
	-> Poll <<Self as Future>::Output>
	{
		let mut_self = self . get_mut ();

		match std::mem::replace (mut_self, Self::Finished)
		{
			Self::Handle {mut handle, shutdown_trigger} => match pin! (&mut handle) . poll (cx)
			{
				Poll::Pending =>
				{
					*mut_self = Self::Handle {handle, shutdown_trigger};
					Poll::Pending
				},
				Poll::Ready (output_result) =>
					Poll::Ready (Self::unwrap_output_result (output_result))
			},
			Self::Finished =>
				panic! ("task handle was polled after output was taken")
		}
	}
}
