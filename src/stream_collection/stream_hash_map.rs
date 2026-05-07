use std::collections::HashMap;
use std::hash::Hash;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::Stream;

pub struct StreamHashMap <K, S>
{
	map: HashMap <K, S>
}

impl <K, S> StreamHashMap <K, S>
where
	K: Eq + Hash,
	S: Stream + Unpin
{
	pub fn new () -> Self
	{
		Self {map: HashMap::new ()}
	}

	pub fn insert (&mut self, key: K, stream: S)
	{
		self . map . insert (key, stream);
	}

	pub fn remove (&mut self, key: &K) -> Option <S>
	{
		self . map . remove (key)
	}

	fn streams (self: Pin <&mut Self>)
	-> impl Iterator <Item = (&K, Pin <&mut S>)>
	{
		// Safety: This code does not modify the shape of the map, and as such
		// does not move any keys.  Shared references to keys are safe, because
		// you cannot move data out of them.
		unsafe { self . get_unchecked_mut () }
			. map
			. iter_mut ()
			. map (|(ref_key, mut_stream)| (ref_key, Pin::new (mut_stream)))
	}
}

impl <K, S> Stream for StreamHashMap <K, S>
where
	K: Clone + Eq + Hash,
	S: Stream + Unpin
{
	type Item = HashMap <K, S::Item>;

	fn poll_next (self: Pin <&mut Self>, cx: &mut Context <'_>)
	-> Poll <Option <Self::Item>>
	{
		let mut changed_map = HashMap::new ();

		for (key, stream) in self . streams ()
		{
			if let Poll::Ready (Some (value)) = stream . poll_next (cx)
			{
				changed_map . insert (key . clone (), value);
			}
		}

		if changed_map . is_empty ()
		{
			Poll::Pending
		}
		else
		{
			Poll::Ready (Some (changed_map))
		}
	}
}
