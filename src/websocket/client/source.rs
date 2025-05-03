use std::fmt::{Debug, Display};
use std::marker::PhantomData;

use futures::SinkExt;
use tungstenite::client::IntoClientRequest;

use crate::task;
use crate::exit_status::WithStatus;
use crate::robust_service::SignallableFallibleServiceFactory;
use crate::service_handle::SignallableServiceHandle;
use crate::websocket::connection::websocket_source;
use crate::websocket::io_format::OutputFormat;

use super::{ConnectionConfig, connect_with_retry};

use crate as compute_graph;

pub struct WebSocketClientSource <OF, R, OS>
{
	output_format: OF,
	connection_config: ConnectionConfig <R>,
	output: OS,
	_of: PhantomData <OF>
}

impl <OF, R, OS> WebSocketClientSource <OF, R, OS>
{
	pub fn new
	(
		output_format: OF,
		connection_config: ConnectionConfig <R>,
		output: OS
	)
	-> Self
	{
		Self
		{
			output_format,
			connection_config,
			output,
			_of: PhantomData::default ()
		}
	}
}

impl <OF, R, OS> SignallableFallibleServiceFactory
for WebSocketClientSource <OF, R, OS>
where
	OF: Clone + OutputFormat + Send + 'static,
	R: Clone + IntoClientRequest + Unpin + Send + Sync,
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
			websocket_source
			(
				self . output_format . clone (),
				self . output . clone (),
				websocket_stream
			)
		)
	}
}
