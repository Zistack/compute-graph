use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::future::FusedFuture;
use pin_project::pin_project;
use tokio::sync::oneshot::Sender;

use super::TaskHandle;

#[pin_project (project = SignallableTaskHandleProjection)]
pub enum SignallableTaskHandle <F>
{
	Future
	{
		#[pin] future: F,
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
where Self: Future
{
	fn abort (self: Pin <&mut Self>)
	{
		if let SignallableTaskHandleProjection::Future {shutdown_trigger, ..}
			= self . project ()
		{
			if let Some (shutdown_trigger) = shutdown_trigger . take ()
			{
				let _ = shutdown_trigger . send (());
			}
		}
	}
}

impl <F> Future for SignallableTaskHandle <F>
where F: Future
{
	type Output = F::Output;

	fn poll (mut self: Pin <&mut Self>, cx: &mut Context) -> Poll <Self::Output>
	{
		match self . as_mut () . project ()
		{
			SignallableTaskHandleProjection::Future {future, ..} =>
				match future . poll (cx)
			{
				Poll::Pending => Poll::Pending,
				Poll::Ready (output) =>
				{
					self . set (Self::Finished);
					Poll::Ready (output)
				}
			},
			SignallableTaskHandleProjection::Finished =>
				panic! ("task handle was polled after output was taken")
		}
	}
}

impl <F> FusedFuture for SignallableTaskHandle <F>
where Self: Future
{
	fn is_terminated (&self) -> bool
	{
		match self
		{
			Self::Future {..} => false,
			Self::Finished => true
		}
	}
}
