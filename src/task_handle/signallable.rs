use std::future::Future;
use std::pin::{Pin, pin};
use std::task::{Context, Poll};

use tokio::sync::oneshot::Sender;

use super::TaskHandle;

pub enum SignallableTaskHandle <F>
{
	Future
	{
		future: F,
		shutdown_trigger: Option <Sender <()>>
	},
	Finished
}

impl <F> SignallableTaskHandle <F>
{
	pub fn new (future: F, shutdown_trigger: Sender <()>) -> Self
	{
		Self::Future {future, shutdown_trigger: Some (shutdown_trigger)}
	}
}

impl <F> TaskHandle for SignallableTaskHandle <F>
where F: Future + Unpin
{
	fn abort (&mut self)
	{
		if let Self::Future {shutdown_trigger, ..} = self
		{
			if let Some (shutdown_trigger) = shutdown_trigger . take ()
			{
				let _ = shutdown_trigger . send (());
			}
		}
	}
}

impl <F> Future for SignallableTaskHandle <F>
where F: Future + Unpin
{
	type Output = F::Output;

	fn poll (self: Pin <&mut Self>, cx: &mut Context)
	-> Poll <Self::Output>
	{
		let mut_self = self . get_mut ();

		match std::mem::replace (mut_self, Self::Finished)
		{
			Self::Future {mut future, shutdown_trigger} => match pin! (&mut future) . poll (cx)
			{
				Poll::Pending =>
				{
					*mut_self = Self::Future {future, shutdown_trigger};
					Poll::Pending
				},
				Poll::Ready (output) => Poll::Ready (output)
			},
			Self::Finished =>
				panic! ("task handle was polled after output was taken")
		}
	}
}
