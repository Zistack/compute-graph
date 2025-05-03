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
	input_format: IF,
	output_format: OF,
	connection_config: ConnectionConfig <R>,
	input: IS,
	output: OS,
	_if: PhantomData <IF>,
	_of: PhantomData <OF>
}

impl <IF, OF, R, IS, OS> WebSocketClientNode <IF, OF, R, IS, OS>
{
	pub fn new
	(
		input_format: IF,
		output_format: OF,
		connection_config: ConnectionConfig <R>,
		input: IS,
		output: OS
	)
	-> Self
	{
		Self
		{
			input_format,
			output_format,
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
	IF: Clone + InputFormat + Send + 'static,
	OF: Clone + OutputFormat + Send + 'static,
	R: Clone + IntoClientRequest + Unpin + Send + Sync,
	IS: Clone + StreamExt + Unpin + Debug + Send + 'static,
	IS::Item: Into <IF::Intermediate> + Send,
	OS: Clone + SinkExt <OF::External> + Unpin + Debug + Send + 'static,
	OF::External: Send,
	OS::Error: Display
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
			websocket_node
			(
				self . input_format . clone (),
				self . output_format . clone (),
				self . input . clone (),
				self . output . clone (),
				websocket_stream
			)
		)
	}
}
