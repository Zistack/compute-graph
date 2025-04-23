use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::future::FusedFuture;
use pin_project::pin_project;
use tokio::task::{JoinHandle, JoinError};

use crate::exit_status::{ExitStatus, ServiceExitStatus};

use super::ServiceHandle;

#[pin_project (project = CancellableServiceHandleProjection)]
pub enum CancellableServiceHandle <T>
{
	Handle (#[pin] JoinHandle <T>),
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
where
	T: Default + ServiceExitStatus,
	Self: Future <Output = T>
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
where T: Default
{
	type Output = T;

	fn poll (mut self: Pin <&mut Self>, cx: &mut Context)
	-> Poll <<Self as Future>::Output>
	{
		match self . as_mut () . project ()
		{
			CancellableServiceHandleProjection::Handle (handle) =>
				match handle . poll (cx)
			{
				Poll::Pending => Poll::Pending,
				Poll::Ready (output_result) =>
				{
					self . set (Self::Taken);
					Poll::Ready (Self::unwrap_output_result (output_result))
				}
			},
			CancellableServiceHandleProjection::Output (_) => unsafe
			{
				if let Self::Output (output) = std::mem::replace
				(
					self . get_unchecked_mut (),
					Self::Taken
				)
				{
					Poll::Ready (output)
				}
				else { unreachable! () }
			},
			CancellableServiceHandleProjection::Taken =>
				panic! ("service handle was polled after output was taken")
		}
	}
}

impl <T> FusedFuture for CancellableServiceHandle <T>
where Self: Future
{
	fn is_terminated (&self) -> bool
	{
		match self
		{
			Self::Handle (_) => false,
			Self::Output (_) => false,
			Self::Taken => true
		}
	}
}
