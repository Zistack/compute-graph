mod node;
pub use node::WebSocketClientNode;
mod node_with_pings;
pub use node_with_pings::WebSocketClientNodeWithPings;
mod source;
pub use source::WebSocketClientSource;
mod source_with_pings;
pub use source_with_pings::WebSocketClientSourceWithPings;
mod sink;
pub use sink::WebSocketClientSink;
mod sink_with_pings;
pub use sink_with_pings::WebSocketClientSinkWithPings;

use rand::rng;
use rand::distr::{Distribution, Uniform};
use tokio::net::TcpStream;
use tokio::sync::oneshot::Receiver;
use tokio::time::{Duration, sleep};
use tokio_tungstenite::{Connector, WebSocketStream, MaybeTlsStream, connect_async_tls_with_config};
use tracing::{Level, event};
use tungstenite::client::IntoClientRequest;
use tungstenite::protocol::WebSocketConfig;

pub struct ConnectionConfig <R>
{
	pub request: R,
	pub stream_config: Option <WebSocketConfig>,
	pub disable_nagle: bool,
	pub connector: Option <Connector>
}

impl <R> ConnectionConfig <R>
{
	pub fn new (request: R) -> Self
	{
		Self
		{
			request,
			stream_config: None,
			disable_nagle: false,
			connector: None
		}
	}

	pub fn with_stream_config (mut self, stream_config: WebSocketConfig) -> Self
	{
		self . stream_config = Some (stream_config);
		self
	}

	pub fn disable_nagle (mut self) -> Self
	{
		self . disable_nagle = true;
		self
	}

	pub fn with_connector (mut self, connector: Connector) -> Self
	{
		self . connector = Some (connector);
		self
	}
}

pub struct PingConfig
{
	pub ping_interval: Duration,
	pub ping_timeout: Duration
}

async fn connect_with_retry <R>
(
	connection_config: &ConnectionConfig <R>,
	shutdown: &mut Receiver <()>
)
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
		match connect_async_tls_with_config
		(
			connection_config . request . clone (),
			connection_config . stream_config . clone (),
			connection_config . disable_nagle,
			connection_config . connector . clone ()
		) . await
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
