use std::fmt::{Debug, Display};
use std::marker::PhantomData;

use futures::{SinkExt, StreamExt};
use rand::rng;
use rand::distr::{Distribution, Uniform};
use tokio::net::TcpStream;
use tokio::sync::oneshot::Receiver;
use tokio::time::{Duration, sleep};
use tokio_tungstenite::{WebSocketStream, MaybeTlsStream, connect_async};
use tracing::{Level, event};
use tungstenite::client::IntoClientRequest;

use crate::task;
use crate::exit_status::WithStatus;
use crate::robust_service::SignallableFallibleServiceFactory;
use crate::service_handle::SignallableServiceHandle;

use super::connection::websocket_node;
use super::io_format::{InputFormat, OutputFormat};

use crate as compute_graph;

async fn connect_with_retry <R> (request: R, shutdown: &mut Receiver <()>)
-> Option <WebSocketStream <MaybeTlsStream <TcpStream>>>
where R: Clone + IntoClientRequest + Unpin
{
	let random_time_distribution = Uniform::new_inclusive
	(
		Duration::from_secs (5) . as_millis () as u64,
		Duration::from_secs (30) . as_millis () as u64
	) . unwrap ();

	loop
	{
		match connect_async (request . clone ()) . await
		{
			Ok ((stream, _)) => return Some (stream),
			Err (connect_error) => event!
			(
				Level::ERROR,
				?connect_error,
				"failed to establish websocket connection"
			)
		}

		let sleep_duration = Duration::from_millis
		(
			random_time_distribution . sample (&mut rng ())
		);

		tokio::select!
		{
			biased;
			_ = &mut *shutdown => return None,
			_ = sleep (sleep_duration) => {}
		}
	}
}

pub struct WebSocketClientNode <IF, OF, R, IS, OS>
{
	request: R,
	input: IS,
	output: OS,
	_if: PhantomData <IF>,
	_of: PhantomData <OF>
}

impl <IF, OF, R, IS, OS> WebSocketClientNode <IF, OF, R, IS, OS>
{
	pub fn new (request: R, input: IS, output: OS) -> Self
	{
		Self
		{
			request,
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
	R: Clone + IntoClientRequest + Unpin + Send,
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
		connect_with_retry (self . request . clone (), &mut shutdown)
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
