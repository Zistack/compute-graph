use bytes::Bytes;
use tracing::{Level, event};
use tungstenite::Message;
use tungstenite::error::Result;

use crate::{
	expand_streams,
	service,
	event_loop_fallible,
	check_break,
	feed,
	flush,
	send
};
use crate::exit_status::{ExitStatus, WithStatus};

use super::io_format::{InputFormat, OutputFormat};

use crate as compute_graph;

#[expand_streams]
#[service (shutdown = shutdown)]
pub async fn shuttle_input_with_pings <F, IS, PS, WS>
(
	_input_format: F,
	inputs: input! (IS -> impl Into <F::Intermediate>),
	ping_bytess: input! (PS -> Bytes),
	websocket: output! (WS <- Message)
)
-> WithStatus <WS>
where F: InputFormat
{
	let status = event_loop_fallible!
	{
		?&mut shutdown,
		ping_bytess -> ping_bytes =>
			check_break! (send! (websocket?, Message::Ping (ping_bytes))),
		inputs -> input.. =>
		{
			if let Some (message) = F::convert (input . into ())
			{
				check_break! (feed! (websocket?, message));
			}
		}
		then check_break! (flush! (websocket?))
	};

	WithStatus::new (websocket, status)
}

#[expand_streams]
#[service (shutdown = shutdown)]
pub async fn shuttle_input <F, IS, WS>
(
	_input_format: F,
	inputs: input! (IS -> impl Into <F::Intermediate>),
	websocket: output! (WS <- Message)
)
-> WithStatus <WS>
where F: InputFormat
{
	let status = event_loop_fallible!
	{
		?&mut shutdown,
		inputs -> input.. =>
		{
			if let Some (message) = F::convert (input . into ())
			{
				check_break! (feed! (websocket?, message))
			}
		}
		then check_break! (flush! (websocket?))
	};

	WithStatus::new (websocket, status)
}

#[expand_streams]
#[service (shutdown = shutdown)]
pub async fn shuttle_output_with_pongs <F, WS, OS, PS>
(
	_output_format: F,
	websocket: input! (WS -> Result <Message>),
	outputs: output! (OS <- F::External),
	pong_bytess: output! (PS <- Bytes)
)
-> WithStatus <WS>
where F: OutputFormat
{
	let status = event_loop_fallible!
	{
		?&mut shutdown,
		websocket? -> message => match message
		{
			Err (ws_error) =>
			{
				event!
				(
					Level::ERROR,
					%ws_error,
					"websocket connection encountered an error"
				);

				break ExitStatus::Spurious;
			}
			Ok (Message::Text (utf8_bytes)) =>
			{
				if let Some (output) = F::convert_text (utf8_bytes)
				{
					check_break!
					(
						send! (outputs, output)
							. map_break (|_| ExitStatus::Clean)
					);
				}
			},
			Ok (Message::Binary (bytes)) =>
			{
				if let Some (output) = F::convert_binary (bytes)
				{
					check_break!
					(
						send! (outputs, output)
							. map_break (|_| ExitStatus::Clean)
					);
				}
			},
			Ok (Message::Ping (_)) => (),
			Ok (Message::Pong (bytes)) => check_break!
			(
				send! (pong_bytess, bytes) . map_break (|_| ExitStatus::Clean)
			),
			Ok (Message::Close (close_frame)) =>
			{
				event!
				(
					Level::INFO,
					?close_frame,
					"websocket connection was closed before shutdown"
				);

				break ExitStatus::Spurious;
			},
			Ok (Message::Frame (_)) => unreachable!
			(
				"websocket stream returned a raw frame"
			)
		}
	};

	WithStatus::new (websocket, status)
}

#[expand_streams]
#[service (shutdown = shutdown)]
pub async fn shuttle_output <F, WS, OS>
(
	_output_format: F,
	websocket: input! (WS -> Result <Message>),
	outputs: output! (OS <- F::External)
)
-> WithStatus <WS>
where F: OutputFormat
{
	let status = event_loop_fallible!
	{
		?&mut shutdown,
		websocket? -> message => match message
		{
			Err (ws_error) =>
			{
				event!
				(
					Level::ERROR,
					%ws_error,
					"websocket connection encountered an error"
				);

				break ExitStatus::Spurious;
			}
			Ok (Message::Text (utf8_bytes)) =>
			{
				if let Some (output) = F::convert_text (utf8_bytes)
				{
					check_break!
					(
						send! (outputs, output)
							. map_break (|_| ExitStatus::Clean)
					);
				}
			},
			Ok (Message::Binary (bytes)) =>
			{
				if let Some (output) = F::convert_binary (bytes)
				{
					check_break!
					(
						send! (outputs, output)
							. map_break (|_| ExitStatus::Clean)
					);
				}
			},
			Ok (Message::Ping (_)) => (),
			Ok (Message::Pong (bytes)) => event!
			(
				Level::WARN,
				pong_bytes = ?bytes,
				"received unsolicited pong frame"
			),
			Ok (Message::Close (close_frame)) =>
			{
				event!
				(
					Level::INFO,
					?close_frame,
					"websocket connection was closed before shutdown"
				);

				break ExitStatus::Spurious;
			},
			Ok (Message::Frame (_)) => unreachable!
			(
				"websocket stream returned a raw frame"
			)
		}
	};

	WithStatus::new (websocket, status)
}

#[expand_streams]
#[service (shutdown = shutdown)]
pub async fn drain_output <WS>
(
	websocket: input! (WS -> Result <Message>)
)
-> WithStatus <WS>
{
	let status = event_loop_fallible!
	{
		?&mut shutdown,
		websocket? -> message => match message
		{
			Err (ws_error) =>
			{
				event!
				(
					Level::ERROR,
					%ws_error,
					"websocket connection encountered an error"
				);

				break ExitStatus::Spurious;
			},
			Ok (Message::Text (utf8_bytes)) => event!
			(
				Level::INFO,
				?utf8_bytes,
				"received superfluous text message"
			),
			Ok (Message::Binary (bytes)) => event!
			(
				Level::INFO,
				?bytes,
				"received superfluous binary message"
			),
			Ok (Message::Ping (_)) => (),
			Ok (Message::Pong (bytes)) => event!
			(
				Level::WARN,
				?bytes,
				"received unsolicited pong frame"
			),
			Ok (Message::Close (close_frame)) =>
			{
				event!
				(
					Level::INFO,
					?close_frame,
					"websocket connection was closed before shutdown"
				);

				break ExitStatus::Spurious;
			},
			Ok (Message::Frame (_)) => unreachable!
			(
				"websocket stream returned a raw frame"
			)
		}
	};

	WithStatus::new (websocket, status)
}
