use std::pin::Pin;
use std::task::{Context, Poll, Waker};

use futures::Stream;

pub struct ReadyItems <'a, S>
{
	stream: Pin <&'a mut S>
}

impl <'a, S> Iterator for ReadyItems <'a, S>
where S: Stream
{
	type Item = Option <S::Item>;

	fn next (&mut self) -> Option <Self::Item>
	{
		let mut ctx = Context::from_waker (Waker::noop ());

		match self . stream . as_mut () . poll_next (&mut ctx)
		{
			Poll::Pending => None,
			Poll::Ready (maybe_item) => Some (maybe_item)
		}
	}
}

pub fn ready_items <S> (stream: Pin <&mut S>) -> ReadyItems <'_, S>
{
	ReadyItems {stream}
}
