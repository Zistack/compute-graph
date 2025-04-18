use std::future::Future;
use std::pin::{Pin, pin};
use std::task::{Context, Poll};

use super::TaskHandle;

pub enum CancellableTaskHandle <F>
{
	Future (F),
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
where F: Future + Unpin
{
	fn abort (&mut self)
	{
		// We drop the future to abort it.
		*self = Self::Finished;
	}
}

impl <F> Future for CancellableTaskHandle <F>
where F: Future + Unpin
{
	type Output = F::Output;

	fn poll (self: Pin <&mut Self>, cx: &mut Context) -> Poll <Self::Output>
	{
		let mut_self = self . get_mut ();

		match std::mem::replace (mut_self, Self::Finished)
		{
			Self::Future (mut future) => match pin! (&mut future) . poll (cx)
			{
				Poll::Pending =>
				{
					*mut_self = Self::Future (future);
					Poll::Pending
				},
				Poll::Ready (output) => Poll::Ready (output)
			}
			Self::Finished =>
				panic! ("task handle was polled after output was taken")
		}
	}
}
