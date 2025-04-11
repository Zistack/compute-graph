use rand::rng;
use rand::distr::{Distribution, Uniform};
use tokio::net::TcpStream;
use tokio::sync::oneshot::Receiver;
use tokio::time::{Duration, sleep};
use tokio_tungstenite::{WebSocketStream, MaybeTlsStream, connect_async};
use tracing::{Level, event};
use tungstenite::client::IntoClientRequest;

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
