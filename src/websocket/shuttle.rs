use bytes::Bytes;
use tracing::{Level, event};
use tungstenite::Message;
use tungstenite::error::Result;

use crate::{expand_streams, service, event_loop_fallible, feed};
use crate::exit_status::{ExitStatus, WithStatus, ShouldTerminateWithStatus};

use super::io_format::{InputFormat, OutputFormat};

use crate as compute_graph;

#[expand_streams]
#[service (shutdown = shutdown)]
pub async fn shuttle_input_with_pings <F, IS, PS, WS>
(
	inputs: input! (IS -> impl Into <F::Intermediate>),
	ping_bytess: input! (PS -> Bytes),
	websocket: output! (WS <- Message)
)
-> WithStatus <WS>
where F: InputFormat
{
	event_loop_fallible!
	{
		_ = &mut shutdown => ExitStatus::Clean,
		ping_bytess -> ping_bytes =>
			feed! (websocket, Message::Ping (ping_bytes)),
		inputs -> input =>
			feed! (websocket?, F::convert (input . into ()))
	}
		. with_value (websocket)
}

#[expand_streams]
#[service (shutdown = shutdown)]
pub async fn shuttle_input <F, IS, WS>
(
	inputs: input! (IS -> impl Into <F::Intermediate>),
	websocket: output! (WS <- Message)
)
-> WithStatus <WS>
where F: InputFormat
{
	event_loop_fallible!
	{
		_ = &mut shutdown => ExitStatus::Clean,
		inputs -> input? => feed! (websocket, F::convert (input . into ()))
	}
		. with_value (websocket)
}

#[expand_streams]
#[service (shutdown = shutdown)]
pub async fn shuttle_output_with_pongs <F, WS, OS, PS>
(
	websocket: input! (WS -> Result <Message>),
	outputs: output! (OS <- F::External),
	pong_bytess: output! (PS <- Bytes)
)
-> WithStatus <WS>
where F: OutputFormat
{
	event_loop_fallible!
	{
		_ = &mut shutdown => ExitStatus::Clean,
		websocket -> message? => match message
		{
			Err (ws_error) =>
			{
				event!
				(
					Level::ERROR,
					%ws_error,
					"websocket connection encountered an error"
				);

				ShouldTerminateWithStatus::from (ExitStatus::Spurious)
			}
			Ok (Message::Text (utf8_bytes)) =>
			{
				if let Some (output) = F::convert_text (utf8_bytes)
				{
					feed! (outputs, output) . into ()
				}
				else
				{
					ShouldTerminateWithStatus::from (None)
				}
			},
			Ok (Message::Binary (bytes)) =>
			{
				if let Some (output) = F::convert_binary (bytes)
				{
					feed! (outputs, output) . into ()
				}
				else
				{
					ShouldTerminateWithStatus::from (None)
				}
			},
			Ok (Message::Ping (_)) => ShouldTerminateWithStatus::from (None),
			Ok (Message::Pong (bytes)) => feed! (pong_bytess, bytes) . into (),
			Ok (Message::Close (close_frame)) =>
			{
				event!
				(
					Level::INFO,
					?close_frame,
					"websocket connection was closed before shutdown"
				);

				ShouldTerminateWithStatus::from (ExitStatus::Spurious)
			},
			Ok (Message::Frame (_)) => unreachable!
			(
				"websocket stream returned a raw frame"
			)
		}
	}
		. with_value (websocket)
}

#[expand_streams]
#[service (shutdown = shutdown)]
pub async fn shuttle_output <F, WS, OS>
(
	websocket: input! (WS -> Result <Message>),
	outputs: output! (OS <- F::External)
)
-> WithStatus <WS>
where F: OutputFormat
{
	event_loop_fallible!
	{
		_ = &mut shutdown => ExitStatus::Clean,
		websocket -> message? => match message
		{
			Err (ws_error) =>
			{
				event!
				(
					Level::ERROR,
					%ws_error,
					"websocket connection encountered an error"
				);

				ShouldTerminateWithStatus::from (ExitStatus::Spurious)
			}
			Ok (Message::Text (utf8_bytes)) =>
			{
				if let Some (output) = F::convert_text (utf8_bytes)
				{
					feed! (outputs, output) . into ()
				}
				else
				{
					ShouldTerminateWithStatus::from (None)
				}
			},
			Ok (Message::Binary (bytes)) =>
			{
				if let Some (output) = F::convert_binary (bytes)
				{
					feed! (outputs, output) . into ()
				}
				else
				{
					ShouldTerminateWithStatus::from (None)
				}
			},
			Ok (Message::Ping (_)) => ShouldTerminateWithStatus::from (None),
			Ok (Message::Pong (bytes)) =>
			{
				event!
				(
					Level::WARN,
					pong_bytes = ?bytes,
					"received unsolicited pong frame"
				);

				ShouldTerminateWithStatus::from (None)
			},
			Ok (Message::Close (close_frame)) =>
			{
				event!
				(
					Level::INFO,
					?close_frame,
					"websocket connection was closed before shutdown"
				);

				ShouldTerminateWithStatus::from (ExitStatus::Spurious)
			},
			Ok (Message::Frame (_)) => unreachable!
			(
				"websocket stream returned a raw frame"
			)
		}
	}
		. with_value (websocket)
}

#[expand_streams]
#[service (shutdown = shutdown)]
pub async fn drain_output <WS>
(
	websocket: input! (WS -> Result <Message>)
)
-> WithStatus <WS>
{
	event_loop_fallible!
	{
		_ = &mut shutdown => ExitStatus::Clean,
		websocket -> message? => match message
		{
			Err (ws_error) =>
			{
				event!
				(
					Level::ERROR,
					%ws_error,
					"websocket connection encountered an error"
				);

				ShouldTerminateWithStatus::from (ExitStatus::Spurious)
			},
			Ok (Message::Text (utf8_bytes)) =>
			{
				event!
				(
					Level::INFO,
					?utf8_bytes,
					"received superfluous text message"
				);

				ShouldTerminateWithStatus::from (None)
			},
			Ok (Message::Binary (bytes)) =>
			{
				event!
				(
					Level::INFO,
					?bytes,
					"received superfluous binary message"
				);

				ShouldTerminateWithStatus::from (None)
			},
			Ok (Message::Ping (_)) => ShouldTerminateWithStatus::from (None),
			Ok (Message::Pong(bytes)) =>
			{
				event!
				(
					Level::WARN,
					"received unsolicited pong frame"
				);

				ShouldTerminateWithStatus::from (None)
			},
			Ok (Message::Close (close_frame)) =>
			{
				event!
				(
					Level::INFO,
					?close_frame,
					"websocket connection was closed before shutdown"
				);

				ShouldTerminateWithStatus::from (ExitStatus::Spurious)
			},
			Ok (Message::Frame (_)) => unreachable!
			(
				"websocket stream returned a raw frame"
			)
		}
	}
		. with_value (websocket)
}
