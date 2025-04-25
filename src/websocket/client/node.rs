use std::fmt::{Debug, Display};
use std::marker::PhantomData;

use futures::{SinkExt, StreamExt};
use tungstenite::client::IntoClientRequest;

use crate::task;
use crate::exit_status::WithStatus;
use crate::robust_service::SignallableFallibleServiceFactory;
use crate::service_handle::SignallableServiceHandle;
use crate::websocket::connection::websocket_node;
use crate::websocket::io_format::{InputFormat, OutputFormat};

use super::{ConnectionConfig, connect_with_retry};

use crate as compute_graph;

pub struct WebSocketClientNode <IF, OF, R, IS, OS>
{
	connection_config: ConnectionConfig <R>,
	input: IS,
	output: OS,
	_if: PhantomData <IF>,
	_of: PhantomData <OF>
}

impl <IF, OF, R, IS, OS> WebSocketClientNode <IF, OF, R, IS, OS>
{
	pub fn new (connection_config: ConnectionConfig <R>, input: IS, output: OS)
	-> Self
	{
		Self
		{
			connection_config,
			input,
			output,
			_if: PhantomData::default (),
			_of: PhantomData::default ()
		}
	}
}

impl <IF, OF, R, IS, OS> SignallableFallibleServiceFactory
for WebSocketClientNode <IF, OF, R, IS, OS>
where
	IF: InputFormat,
	OF: OutputFormat,
	R: Clone + IntoClientRequest + Unpin + Send + Sync,
	IS: Clone + StreamExt + Unpin + Debug + Send + 'static,
	IS::Item: Into <IF::Intermediate> + Send,
	OS: Clone + SinkExt <OF::External> + Unpin + Debug + Send + 'static,
	OF::External: Send,
	OS::Error: Display,
	Self: Send
{
	#[task (shutdown = shutdown)]
	async fn construct (&mut self)
	-> Option <SignallableServiceHandle <WithStatus>>
	{
		connect_with_retry (&self . connection_config, &mut shutdown)
			. await
			. map
		(
			|websocket_stream|
			websocket_node::<IF, OF, IS, OS, _>
			(
				self . input . clone (),
				self . output . clone (),
				websocket_stream
			)
		)
	}
}
