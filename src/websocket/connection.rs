use std::fmt::Debug;

use futures::StreamExt;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::time::Duration;
use tokio::sync::mpsc;
use tokio_tungstenite::WebSocketStream;
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::sync::PollSender;
use tungstenite::Message;

use crate::{expand_streams, service, join_services, send};
use crate::exit_status::{ExitStatus, ServiceExitStatus, WithStatus};

use super::io_format::{InputFormat, OutputFormat};
use super::shuttle::*;
use super::keepalive::*;

use crate as compute_graph;

#[expand_streams]
#[service (shutdown = shutdown)]
pub async fn websocket_node_with_pings <IF, OF, IS, OS, S>
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

		send! (websocket_sink, Message::Close (None));

		WithStatus::from (ExitStatus::Clean)
	}
}

#[expand_streams]
#[service (shutdown = shutdown)]
pub async fn websocket_node <IF, OF, IS, OS, S>
(
	inputs: input! (IS -> impl Into <IF::Intermediate>),
	outputs: output! (OS <- OF::External),
	websocket: WebSocketStream <S>
)
-> WithStatus
where
	IF: InputFormat,
	OF: OutputFormat,
	S: AsyncRead + AsyncWrite + Unpin + Debug
{
	let (websocket_sink, websocket_stream) = websocket . split ();

	let shuttle_input_handle =
		shuttle_input::<IF, _, _> (inputs, websocket_sink);
	let shuttle_output_handle =
		shuttle_output::<OF, _, _> (websocket_stream, outputs);

	let (input_report, output_report) = join_services!
	(
		?shutdown,
		shuttle_input_handle,
		shuttle_output_handle
	);

	if input_report . status_spurious () || output_report . status_spurious ()
	{
		WithStatus::from (ExitStatus::Spurious)
	}
	else
	{
		let (mut websocket_sink, _) = input_report . split ();

		send! (websocket_sink, Message::Close (None));

		WithStatus::from (ExitStatus::Clean)
	}
}

#[expand_streams]
#[service (shutdown = shutdown)]
pub async fn websocket_source_with_pings <OF, OS, S>
(
	outputs: output! (OS <- OF::External),
	websocket: WebSocketStream <S>,
	ping_interval: Duration,
	ping_timeout: Duration
)
-> WithStatus
where
	OF: OutputFormat,
	S: AsyncRead + AsyncWrite + Unpin + Debug
{
	let (websocket_sink, websocket_stream) = websocket . split ();

	let (pong_sender, pong_receiver) = mpsc::channel (1);

	let shuttle_output_handle = shuttle_output_with_pongs::<OF, _, _, _>
	(
		websocket_stream,
		outputs,
		PollSender::new (pong_sender)
	);

	let keepalive_handle = keepalive_direct_pings
	(
		websocket_sink,
		ReceiverStream::new (pong_receiver),
		ping_interval,
		ping_timeout
	);

	let (output_report, keepalive_report) = join_services!
	(
		?shutdown,
		shuttle_output_handle,
		keepalive_handle
	);

	if output_report . status_spurious () || keepalive_report . status_spurious ()
	{
		WithStatus::from (ExitStatus::Spurious)
	}
	else
	{
		let (mut websocket_sink, _) = keepalive_report . split ();

		send! (websocket_sink, Message::Close (None));

		WithStatus::from (ExitStatus::Clean)
	}
}

#[expand_streams]
#[service (shutdown = shutdown)]
pub async fn websocket_source <OF, OS, S>
(
	outputs: output! (OS <- OF::External),
	websocket: WebSocketStream <S>
)
-> WithStatus
where
	OF: OutputFormat,
	S: AsyncRead + AsyncWrite + Unpin + Debug
{
	let (mut websocket_sink, websocket_stream) = websocket . split ();

	let shuttle_output_handle =
		shuttle_output::<OF, _, _> (websocket_stream, outputs);

	let (output_report,) = join_services! (?shutdown, shuttle_output_handle);

	if output_report . status_spurious ()
	{
		WithStatus::from (ExitStatus::Spurious)
	}
	else
	{
		send! (websocket_sink, Message::Close (None));

		WithStatus::from (ExitStatus::Clean)
	}
}

#[expand_streams]
#[service (shutdown = shutdown)]
pub async fn websocket_sink_with_pings <IF, IS, S>
(
	inputs: input! (IS -> impl Into <IF::Intermediate>),
	websocket: WebSocketStream <S>,
	ping_interval: Duration,
	ping_timeout: Duration
)
-> WithStatus
where
	IF: InputFormat,
	S: AsyncRead + AsyncWrite + Unpin + Debug
{
	let (websocket_sink, websocket_stream) = websocket . split ();

	let (ping_sender, ping_receiver) = mpsc::channel (1);

	let shuttle_input_handle = shuttle_input_with_pings::<IF, _, _, _>
	(
		inputs,
		ReceiverStream::new (ping_receiver),
		websocket_sink
	);

	let keepalive_handle = keepalive_direct_pongs
	(
		PollSender::new (ping_sender),
		websocket_stream,
		ping_interval,
		ping_timeout
	);

	let (input_report, keepalive_report) = join_services!
	(
		?shutdown,
		shuttle_input_handle,
		keepalive_handle
	);

	if input_report . status_spurious () || keepalive_report . status_spurious ()
	{
		WithStatus::from (ExitStatus::Spurious)
	}
	else
	{
		let (mut websocket_sink, _) = input_report . split ();

		send! (websocket_sink, Message::Close (None));

		WithStatus::from (ExitStatus::Clean)
	}
}

#[expand_streams]
#[service (shutdown = shutdown)]
pub async fn websocket_sink <IF, IS, S>
(
	inputs: input! (IS -> impl Into <IF::Intermediate>),
	websocket: WebSocketStream <S>
)
-> WithStatus
where
	IF: InputFormat,
	S: AsyncRead + AsyncWrite + Unpin + Debug
{
	let (websocket_sink, websocket_stream) = websocket . split ();

	let shuttle_input_handle =
		shuttle_input::<IF, _, _> (inputs, websocket_sink);
	let drain_handle = drain_output (websocket_stream);

	let (input_report, output_report) = join_services!
	(
		?shutdown,
		shuttle_input_handle,
		drain_handle
	);

	if input_report . status_spurious () || output_report . status_spurious ()
	{
		WithStatus::from (ExitStatus::Spurious)
	}
	else
	{
		let (mut websocket_sink, _) = input_report . split ();

		send! (websocket_sink, Message::Close (None));

		WithStatus::from (ExitStatus::Clean)
	}
}
