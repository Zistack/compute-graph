use std::fmt::Debug;

use futures::StreamExt;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::time::Duration;
use tokio::sync::mpsc;
use tokio_tungstenite::WebSocketStream;
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::sync::PollSender;
use tungstenite::Message;

use crate::{expand_streams, service, join_services, feed};
use crate::exit_status::{ExitStatus, ServiceExitStatus, WithStatus};

use super::io_format::{InputFormat, OutputFormat};
use super::shuttle::*;
use super::keepalive::*;

use crate as compute_graph;

#[expand_streams]
#[service (shutdown = shutdown)]
pub async fn websocket_node_wtih_pings <IS, IF, OS, OF, S>
(
	inputs: input! (IS -> impl Into <IF::Intermediate>),
	outputs: output! (OS <- OF::External),
	websocket: WebSocketStream <S>,
	ping_interval: Duration,
	ping_timeout: Duration
)
-> WithStatus
where
	IF: InputFormat,
	OF: OutputFormat,
	S: AsyncRead + AsyncWrite + Unpin + Debug
{
	let (websocket_sink, websocket_stream) = websocket . split ();

	// Buffer size doesn't need to be big here.
	let (ping_sender, ping_receiver) = mpsc::channel (1);
	let (pong_sender, pong_receiver) = mpsc::channel (1);

	let shuttle_input_handle = shuttle_input_with_pings::<IF, _, _, _>
	(
		inputs,
		ReceiverStream::new (ping_receiver),
		websocket_sink
	);

	let shuttle_output_handle = shuttle_output_with_pongs::<OF, _, _, _>
	(
		websocket_stream,
		outputs,
		PollSender::new (pong_sender)
	);

	let keepalive_handle = keepalive
	(
		PollSender::new (ping_sender),
		ReceiverStream::new (pong_receiver),
		ping_interval,
		ping_timeout
	);

	let (input_report, output_report, keepalive_report) = join_services!
	(
		?shutdown,
		shuttle_input_handle,
		shuttle_output_handle,
		keepalive_handle
	);

	if input_report . status_spurious ()
		|| output_report . status_spurious ()
		|| keepalive_report . status_spurious ()
	{
		WithStatus::from (ExitStatus::Spurious)
	}
	else
	{
		let (mut websocket_sink, _) = input_report . split ();

		feed! (websocket_sink?, Message::Close (None));

		WithStatus::from (ExitStatus::Clean)
	}
}
