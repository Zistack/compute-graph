use std::fmt::Debug;
use std::marker::PhantomData;

use futures::StreamExt;
use tungstenite::client::IntoClientRequest;

use crate::task;
use crate::exit_status::WithStatus;
use crate::robust_service::SignallableFallibleServiceFactory;
use crate::service_handle::SignallableServiceHandle;
use crate::websocket::connection::websocket_sink_with_pings;
use crate::websocket::io_format::InputFormat;

use super::{ConnectionConfig, PingConfig, connect_with_retry};

use crate as compute_graph;

pub struct WebSocketClientSinkWithPings <IF, R, IS>
{
	input_format: IF,
	connection_config: ConnectionConfig <R>,
	input: IS,
	ping_config: PingConfig,
	_if: PhantomData <IF>
}

impl <IF, R, IS> WebSocketClientSinkWithPings <IF, R, IS>
{
	pub fn new
	(
		input_format: IF,
		connection_config: ConnectionConfig <R>,
		input: IS,
		ping_config: PingConfig
	)
	-> Self
	{
		Self
		{
			input_format,
			connection_config,
			input,
			ping_config,
			_if: PhantomData::default ()
		}
	}
}

impl <IF, R, IS> SignallableFallibleServiceFactory
for WebSocketClientSinkWithPings <IF, R, IS>
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
			websocket_sink_with_pings
			(
				self . input_format . clone (),
				self . input . clone (),
				websocket_stream,
				self . ping_config . ping_interval,
				self . ping_config . ping_timeout
			)
		)
	}
}
