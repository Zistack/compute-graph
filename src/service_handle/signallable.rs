use std::future::Future;
use std::pin::{Pin, pin};
use std::task::{Context, Poll};

use futures::future::FusedFuture;
use tokio::sync::oneshot::Sender;
use tokio::task::{JoinHandle, JoinError};

use crate::exit_status::{ExitStatus, ServiceExitStatus};

use super::ServiceHandle;

pub enum SignallableServiceHandle <T>
{
	Handle
	{
		handle: JoinHandle <T>,
		shutdown_trigger: Option <Sender <()>>
	},
	Output (T),
	Taken
}

impl <T> SignallableServiceHandle <T>
{
	pub fn new (handle: JoinHandle <T>, shutdown_trigger: Sender <()>) -> Self
	{
		Self::Handle {handle, shutdown_trigger: Some (shutdown_trigger)}
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

				panic! ("service was cancelled")
			}
		}
	}
}

impl <T> Unpin for SignallableServiceHandle <T>
{
}

impl <T> ServiceHandle for SignallableServiceHandle <T>
where
	T: ServiceExitStatus,
	Self: Future <Output = T>
{
	fn shutdown (&mut self)
	{
		if let Self::Handle {shutdown_trigger, ..} = self
		{
			if let Some (shutdown_trigger) = shutdown_trigger . take ()
			{
				let _ = shutdown_trigger . send (());
			}
		}
	}

	async fn exit_status (&mut self) -> Option <ExitStatus>
	{
		let output_result = match self
		{
			Self::Handle {handle, ..} => handle . await,
			Self::Output (output) => return Some (output . exit_status ()),
			Self::Taken => return None
		};

		let output = Self::unwrap_output_result (output_result);

		let exit_status = output . exit_status ();

		*self = Self::Output (output);

		Some (exit_status)
	}

	fn take_output (&mut self) -> Option <<Self as Future>::Output>
	{
		match std::mem::replace (self, Self::Taken)
		{
			handle @ Self::Handle {..} =>
			{
				*self = handle;
				None
			},
			Self::Output (output) => Some (output),
			Self::Taken => None
		}
	}
}

impl <T> Future for SignallableServiceHandle <T>
{
	type Output = T;

	fn poll (mut self: Pin <&mut Self>, cx: &mut Context)
	-> Poll <<Self as Future>::Output>
	{
		match self . as_mut () . get_mut ()
		{
			Self::Handle {handle, ..} =>
				match pin! (handle) . poll (cx)
			{
				Poll::Pending => Poll::Pending,
				Poll::Ready (output_result) =>
				{
					self . set (Self::Taken);
					Poll::Ready (Self::unwrap_output_result (output_result))
				}
			},
			Self::Output (_) =>
			{
				if let Self::Output (output) = std::mem::replace
				(
					self . get_mut (),
					Self::Taken
				)
				{
					Poll::Ready (output)
				}
				else { unreachable! () }
			}
			Self::Taken =>
				panic! ("service handle was polled after output was taken")
		}
	}
}

impl <T> FusedFuture for SignallableServiceHandle <T>
where Self: Future
{
	fn is_terminated (&self) -> bool
	{
		match self
		{
			Self::Handle {..} => false,
			Self::Output (_) => false,
			Self::Taken => true
		}
	}
}
