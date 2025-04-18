use std::future::Future;
use std::pin::{Pin, pin};
use std::task::{Context, Poll};

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

impl <T> ServiceHandle for SignallableServiceHandle <T>
where T: Unpin + ServiceExitStatus
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
where T: Unpin
{
	type Output = T;

	fn poll (self: Pin <&mut Self>, cx: &mut Context)
	-> Poll <<Self as Future>::Output>
	{
		let mut_self = self . get_mut ();

		match std::mem::replace (mut_self, Self::Taken)
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
			Self::Output (output) => Poll::Ready (output),
			Self::Taken =>
				panic! ("service handle was polled after output was taken")
		}
	}
}
