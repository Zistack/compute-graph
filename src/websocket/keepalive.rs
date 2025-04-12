use bytes::Bytes;
use tokio::time::{Duration, MissedTickBehavior, interval, sleep};
use tokio::sync::oneshot::Receiver;
use tracing::{Level, event};
use tungstenite::Message;

use crate::{
	expand_streams,
	service,
	select_fallible,
	event_loop_fallible,
	feed,
	return_if_should_terminate
};
use crate::exit_status::{
	ExitStatus,
	ServiceShouldTerminate,
	WithStatus,
	ShouldTerminateWithStatus
};

use crate as compute_graph;

#[expand_streams]
pub async fn ping <PIS, POS>
(
	shutdown: &mut Receiver <()>,
	pings: output! (PIS <- Bytes),
	pongs: input! (POS -> Bytes),
	ping_bytes: Bytes,
	ping_timeout: Duration
)
-> ShouldTerminateWithStatus
{
	return_if_should_terminate!
	{
		feed! (pings, ping_bytes . clone ())
	}

	select_fallible!
	{
		_ = shutdown => ShouldTerminateWithStatus::from (ExitStatus::Clean),
		pongs -> pong_bytes => if pong_bytes != ping_bytes
		{
			event!
			(
				Level::ERROR,
				?ping_bytes,
				?pong_bytes,
				"pong frame bytes did not match ping frame bytes"
			);

			ShouldTerminateWithStatus::from (ExitStatus::Spurious)
		}
		else
		{
			ShouldTerminateWithStatus::from (None)
		},
		_ = sleep (ping_timeout) =>
		{
			event!
			(
				Level::ERROR,
				"websocket connection timed out"
			);

			ShouldTerminateWithStatus::from (ExitStatus::Spurious)
		}
	}
}

#[expand_streams]
#[service (shutdown = shutdown)]
pub async fn keepalive <PIS, POS>
(
	pings: output! (PIS <- Bytes),
	pongs: input! (POS -> Bytes),
	ping_interval: Duration,
	ping_timeout: Duration
)
-> WithStatus
{
	let mut ping_clock = interval (ping_interval);
	ping_clock . set_missed_tick_behavior (MissedTickBehavior::Delay);

	let mut ping_counter: u32 = 0;

	event_loop_fallible!
	{
		_ = &mut shutdown => ShouldTerminateWithStatus::from (ExitStatus::Clean),
		_ = ping_clock . tick () =>
		{
			let ping_bytes = Bytes::from_owner (ping_counter . to_be_bytes ());
			ping_counter += 1;

			ping
			(
				&mut shutdown,
				&mut pings,
				&mut pongs,
				ping_bytes,
				ping_timeout
			) . await
		}
	}
}

#[expand_streams]
pub async fn ping_direct_input <PIS, WS>
(
	shutdown: &mut Receiver <()>,
	pings: output! (PIS <- Bytes),
	websocket: input! (WS -> Message),
	ping_bytes: Bytes,
	ping_timeout: Duration
)
-> ShouldTerminateWithStatus
{
	return_if_should_terminate!
	{
		feed! (pings, ping_bytes . clone ())
	}

	select_fallible!
	{
		_ = shutdown => ShouldTerminateWithStatus::from (ExitStatus::Clean),
		websocket -> message => match message
		{
			Message::Text (utf8_bytes) =>
			{
				event!
				(
					Level::INFO,
					?utf8_bytes,
					"received superfluous text message"
				);

				ShouldTerminateWithStatus::from (None)
			},
			Message::Binary (bytes) =>
			{
				event!
				(
					Level::INFO,
					?bytes,
					"received superfluous binary message"
				);

				ShouldTerminateWithStatus::from (None)
			},
			Message::Ping (_) => ShouldTerminateWithStatus::from (None),
			Message::Pong (pong_bytes) => if pong_bytes != ping_bytes
			{
				event!
				(
					Level::ERROR,
					?ping_bytes,
					?pong_bytes,
					"pong frame bytes did not match ping frame bytes"
				);

				ShouldTerminateWithStatus::from (ExitStatus::Spurious)
			}
			else
			{
				ShouldTerminateWithStatus::from (None)
			},
			Message::Close (close_frame) =>
			{
				event!
				(
					Level::INFO,
					?close_frame,
					"websocket connection was closed before shutdown"
				);

				ShouldTerminateWithStatus::from (ExitStatus::Spurious)
			},
			Message::Frame (_) => unreachable!
			(
				"websocket stream returned raw frame"
			)
		},
		_ = sleep (ping_timeout) =>
		{
			event!
			(
				Level::ERROR,
				"websocket connection timed out"
			);

			ShouldTerminateWithStatus::from (ExitStatus::Spurious)
		}
	}
}

#[expand_streams]
#[service (shutdown = shutdown)]
pub async fn keepalive_direct_input <PIS, WS>
(
	pings: output! (PIS <- Bytes),
	websocket: input! (WS -> Message),
	ping_interval: Duration,
	ping_timeout: Duration
)
-> WithStatus <WS>
{
	let mut ping_clock = interval (ping_interval);
	ping_clock . set_missed_tick_behavior (MissedTickBehavior::Delay);

	let mut ping_counter: u32 = 0;

	event_loop_fallible!
	{
		_ = &mut shutdown => ShouldTerminateWithStatus::from (ExitStatus::Clean),
		_ = ping_clock . tick () =>
		{
			let ping_bytes = Bytes::from_owner (ping_counter . to_be_bytes ());
			ping_counter += 1;

			ping_direct_input
			(
				&mut shutdown,
				&mut pings,
				&mut websocket,
				ping_bytes,
				ping_timeout
			) . await
		}
	}
		. with_value (websocket)
}

#[expand_streams]
pub async fn ping_direct_output <WS, POS>
(
	shutdown: &mut Receiver <()>,
	websocket: output! (WS <- Message),
	pongs: input! (POS -> Bytes),
	ping_bytes: Bytes,
	ping_timeout: Duration
)
-> ShouldTerminateWithStatus
{
	return_if_should_terminate!
	{
		feed! (websocket?, Message::Ping (ping_bytes . clone ()))
	}

	select_fallible!
	{
		_ = shutdown => ShouldTerminateWithStatus::from (ExitStatus::Clean),
		pongs -> pong_bytes => if pong_bytes != ping_bytes
		{
			event!
			(
				Level::ERROR,
				?ping_bytes,
				?pong_bytes,
				"pong frame bytes did not match ping frame bytes"
			);

			ShouldTerminateWithStatus::from (ExitStatus::Spurious)
		}
		else
		{
			ShouldTerminateWithStatus::from (None)
		},
		_ = sleep (ping_timeout) =>
		{
			event!
			(
				Level::ERROR,
				"websocket connection timed out"
			);

			ShouldTerminateWithStatus::from (ExitStatus::Spurious)
		}
	}
}

#[expand_streams]
#[service (shutdown = shutdown)]
pub async fn keepalive_direct_output <WS, POS>
(
	websocket: output! (WS <- Message),
	pongs: input! (POS -> Bytes),
	ping_interval: Duration,
	ping_timeout: Duration
)
-> WithStatus <WS>
{
	let mut ping_clock = interval (ping_interval);
	ping_clock . set_missed_tick_behavior (MissedTickBehavior::Delay);

	let mut ping_counter: u32 = 0;

	event_loop_fallible!
	{
		_ = &mut shutdown => ShouldTerminateWithStatus::from (ExitStatus::Clean),
		_ = ping_clock . tick () =>
		{
			let ping_bytes = Bytes::from_owner (ping_counter . to_be_bytes ());
			ping_counter += 1;

			ping_direct_output
			(
				&mut shutdown,
				&mut websocket,
				&mut pongs,
				ping_bytes,
				ping_timeout
			) . await
		}
	}
		. with_value (websocket)
}
