use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use pin_project::pin_project;

use super::TaskHandle;

#[pin_project (project = CancellableTaskHandleProjection)]
pub enum CancellableTaskHandle <F>
{
	Future (#[pin] F),
	Aborted,
	Finished
}

impl <F> CancellableTaskHandle <F>
{
	pub fn new (future: F) -> Self
	{
		Self::Future (future)
	}
}

impl <F> TaskHandle for CancellableTaskHandle <F>
where Self: Future
{
	fn abort (&mut self)
	{
		// We drop the future to abort it, and mark the handle as aborted so
		// that awaiting it does the right thing.
		*self = Self::Aborted;
	}
}

impl <F> Future for CancellableTaskHandle <F>
where
	F: Future,
	F::Output: Default
{
	type Output = F::Output;

	fn poll (mut self: Pin <&mut Self>, cx: &mut Context) -> Poll <Self::Output>
	{
		match self . as_mut () . project ()
		{
			CancellableTaskHandleProjection::Future (future) =>
				match future . poll (cx)
			{
				Poll::Pending => Poll::Pending,
				Poll::Ready (output) =>
				{
					self . set (Self::Finished);
					Poll::Ready (output)
				}
			},
			CancellableTaskHandleProjection::Aborted =>
			{
				self . set (Self::Finished);
				Poll::Ready (F::Output::default ())
			}
			CancellableTaskHandleProjection::Finished =>
				panic! ("task handle was polled after output was taken")
		}
	}
}
