use std::fmt::Debug;
use std::marker::PhantomData;

use futures::StreamExt;
use tungstenite::client::IntoClientRequest;

use crate::task;
use crate::exit_status::WithStatus;
use crate::robust_service::SignallableFallibleServiceFactory;
use crate::service_handle::SignallableServiceHandle;
use crate::websocket::connection::websocket_sink;
use crate::websocket::io_format::InputFormat;

use super::{ConnectionConfig, connect_with_retry};

use crate as compute_graph;

pub struct WebSocketClientSink <IF, R, IS>
{
	input_format: IF,
	connection_config: ConnectionConfig <R>,
	input: IS,
	_if: PhantomData <IF>
}

impl <IF, R, IS> WebSocketClientSink <IF, R, IS>
{
	pub fn new
	(
		input_format: IF,
		connection_config: ConnectionConfig <R>,
		input: IS
	)
	-> Self
	{
		Self
		{
			input_format,
			connection_config,
			input,
			_if: PhantomData::default ()
		}
	}
}

impl <IF, R, IS> SignallableFallibleServiceFactory
for WebSocketClientSink <IF, R, IS>
where
	IF: Clone + InputFormat + Send + 'static,
	R: Clone + IntoClientRequest + Unpin + Send + Sync,
	IS: Clone + StreamExt + Unpin + Debug + Send + 'static,
	IS::Item: Into <IF::Intermediate> + Send
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
			websocket_sink
			(
				self . input_format . clone (),
				self . input . clone (),
				websocket_stream
			)
		)
	}
}
