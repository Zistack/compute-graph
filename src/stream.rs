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
