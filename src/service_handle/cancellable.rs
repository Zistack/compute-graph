use std::future::Future;
use std::pin::{Pin, pin};
use std::task::{Context, Poll};

use tokio::task::{JoinHandle, JoinError};

use crate::exit_status::{ExitStatus, ServiceExitStatus};

use super::ServiceHandle;

pub enum CancellableServiceHandle <T>
{
	Handle (JoinHandle <T>),
	Output (T),
	Taken
}

impl <T> CancellableServiceHandle <T>
{
	pub fn new (handle: JoinHandle <T>) -> Self
	{
		Self::Handle (handle)
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

impl <T> ServiceHandle for CancellableServiceHandle <T>
where T: Unpin + Default + ServiceExitStatus
{
	fn shutdown (&mut self)
	{
		if let Self::Handle (handle) = self
		{
			handle . abort ();
		}
	}

	async fn exit_status (&mut self) -> Option <ExitStatus>
	{
		let output_result = match self
		{
			Self::Handle (handle) => handle . await,
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
			Self::Handle (handle) =>
			{
				*self = Self::Handle (handle);
				None
			}
			Self::Output (output) => Some (output),
			Self::Taken => None
		}
	}
}

impl <T> Future for CancellableServiceHandle <T>
where T: Unpin + Default
{
	type Output = T;

	fn poll (self: Pin <&mut Self>, cx: &mut Context)
	-> Poll <<Self as Future>::Output>
	{
		let mut_self = self . get_mut ();

		match std::mem::replace (mut_self, Self::Taken)
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
			},
			Self::Output (output) => Poll::Ready (output),
			Self::Taken =>
				panic! ("service handle was polled after output was taken")
		}
	}
}
