mod ready_items;
pub use ready_items::*;

use std::fmt::{Debug, Display};

use futures::{Sink, Stream};

pub fn mpsc <T> (buffer: usize)
-> (
	impl Clone + Sink <T, Error: Display> + Unpin + Debug + Send + 'static,
	impl Stream <Item = T> + Unpin + Debug + Send + 'static
)
where T: Debug + Send + 'static
{
	let (sender, receiver) = tokio::sync::mpsc::channel (buffer);

	(
		tokio_util::sync::PollSender::new (sender),
		tokio_stream::wrappers::ReceiverStream::new (receiver)
	)
}

pub fn watch <T> (initial_value: T)
-> (
	tokio::sync::watch::Sender <T>,
	impl Stream <Item = T> + Unpin + Debug + Send + 'static
)
where T: Clone + Send + Sync + 'static
{
	let (sender, receiver) = tokio::sync::watch::channel (initial_value);

	(sender, tokio_stream::wrappers::WatchStream::new (receiver))
}
