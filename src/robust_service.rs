use std::future::Future;

use tokio::sync::{oneshot, watch};
use tokio::time::{Duration, sleep};

use crate::{service, select, event_loop};
use crate::exit_status::{AlwaysClean, ShouldTerminateClean};
use crate::service_handle::ServiceHandle;
use crate::service_state::ServiceState;

use crate as compute_graph;

async fn replace_down_service <C, S>
(
	constructor_handle: C,
	service_handle: &mut S,
	state_channel: &mut watch::Sender <ServiceState>
)
where
	C: ServiceHandle + Future <Output = AlwaysClean <S>> + Unpin,
	S: ServiceHandle
{
	let new_service_handle = constructor_handle . await . into_value ();

	let mut old_service_handle =
		std::mem::replace (service_handle, new_service_handle);

	state_channel . send_replace (ServiceState::Up);

	old_service_handle . shutdown ();
	old_service_handle . await;
}

async fn replace_service <C, S>
(
	mut constructor_handle: C,
	service_handle: &mut S,
	state_channel: &mut watch::Sender <ServiceState>
)
where
	C: ServiceHandle + Future <Output = AlwaysClean <S>> + Unpin,
	S: ServiceHandle + Unpin
{
	select!
	{
		_ = &mut *service_handle =>
		{
			state_channel . send_replace (ServiceState::Down);

			replace_down_service
			(
				constructor_handle,
				service_handle,
				state_channel
			) . await
		},
		constructor_result = &mut constructor_handle =>
		{
			let new_service_handle = constructor_result . into_value ();
			let mut old_service_handle =
				std::mem::replace (service_handle, new_service_handle);

			// State UP (though we should still be UP at this point)

			old_service_handle . shutdown ();
			old_service_handle . await;
		}
	};
}

#[service]
pub async fn robust_service <F, C, S>
(
	mut constructor: F,
	mut state_channel: watch::Sender <ServiceState>
)
-> AlwaysClean
where
	F: FnMut () -> C,
	C: ServiceHandle + Future <Output = AlwaysClean <S>> + Send + Unpin,
	S: ServiceHandle + Send + Unpin,
	S::Output: Send
{
	let mut service_handle = constructor () . await . into_value ();

	state_channel . send_replace (ServiceState::Up);

	event_loop!
	{
		_ = &mut service_handle =>
		{
			state_channel . send_replace (ServiceState::Down);

			replace_down_service
			(
				constructor (),
				&mut service_handle,
				&mut state_channel
			) . await
		}
	}
}

#[service]
pub async fn robust_service_with_preemptive_replacement <F, C, S>
(
	mut constructor: F,
	mut state_channel: watch::Sender <ServiceState>,
	replacement_interval: Duration
)
-> AlwaysClean
where
	F: FnMut () -> C,
	C: ServiceHandle + Future <Output = AlwaysClean <S>> + Send + Unpin,
	S: ServiceHandle + Send + Unpin,
	S::Output: Send
{
	let mut service_handle = constructor () . await . into_value ();

	state_channel . send_replace (ServiceState::Up);

	event_loop!
	{
		_ = &mut service_handle =>
		{
			state_channel .send_replace (ServiceState::Down);

			replace_down_service
			(
				constructor (),
				&mut service_handle,
				&mut state_channel
			) . await
		},
		_ = sleep (replacement_interval) => replace_service
		(
			constructor (),
			&mut service_handle,
			&mut state_channel
		) . await
	}
}

async fn replace_down_service_with_shutdown <C, S>
(
	shutdown: &mut oneshot::Receiver <()>,
	mut constructor_handle: C,
	service_handle: &mut S,
	state_channel: &mut watch::Sender <ServiceState>
)
-> ShouldTerminateClean
where
	C: ServiceHandle + Future <Output = AlwaysClean <S>> + Unpin,
	S: ServiceHandle
{
	select!
	{
		_ = &mut *shutdown => ShouldTerminateClean::from (true),
		constructor_result = &mut constructor_handle =>
		{
			let new_service_handle = constructor_result . into_value ();
			let mut old_service_handle =
				std::mem::replace (service_handle, new_service_handle);

			state_channel . send_replace (ServiceState::Up);

			old_service_handle . shutdown ();
			old_service_handle . await;
		}
	}
}

async fn replace_service_with_shutdown <C, S>
(
	shutdown: &mut oneshot::Receiver <()>,
	mut constructor_handle: C,
	service_handle: &mut S,
	state_channel: &mut watch::Sender <ServiceState>
)
-> ShouldTerminateClean
where
	C: ServiceHandle + Future <Output = AlwaysClean <S>> + Unpin,
	S: ServiceHandle + Unpin
{
	select!
	{
		_ = &mut *shutdown => ShouldTerminateClean::from (true),
		_ = &mut *service_handle =>
		{
			state_channel . send_replace (ServiceState::Down);

			replace_down_service_with_shutdown
			(
				shutdown,
				constructor_handle,
				service_handle,
				state_channel
			) . await
		},
		constructor_result = &mut constructor_handle =>
		{
			let new_service_handle = constructor_result . into_value ();
			let mut old_service_handle =
				std::mem::replace (service_handle, new_service_handle);

			// State UP (though we should still be UP at this point)

			old_service_handle . shutdown ();
			old_service_handle . await;
		}
	}
}

#[service (shutdown = shutdown)]
pub async fn robust_service_with_shutdown <F, C, S>
(
	mut constructor: F,
	mut state_channel: watch::Sender <ServiceState>
)
-> AlwaysClean
where
	F: FnMut () -> C,
	C: ServiceHandle + Future <Output = AlwaysClean <S>> + Send + Unpin,
	S: ServiceHandle + Send + Unpin,
	S::Output: Send
{
	let mut service_handle = tokio::select!
	{
		_ = &mut shutdown => return AlwaysClean::new (()),
		constructor_result = constructor () =>
			constructor_result . into_value ()
	};

	state_channel . send_replace (ServiceState::Up);

	event_loop!
	{
		_ = &mut shutdown => ShouldTerminateClean::from (true),
		_ = &mut service_handle =>
		{
			state_channel . send_replace (ServiceState::Down);

			replace_down_service_with_shutdown
			(
				&mut shutdown,
				constructor (),
				&mut service_handle,
				&mut state_channel
			) . await
		}
	};

	service_handle . shutdown ();
	service_handle . await;

	AlwaysClean::new (())
}

#[service (shutdown = shutdown)]
pub async fn robust_service_with_shutdown_and_preemptive_replacement <F, C, S>
(
	mut constructor: F,
	mut state_channel: watch::Sender <ServiceState>,
	replacement_interval: Duration
)
-> AlwaysClean
where
	F: FnMut () -> C,
	C: ServiceHandle + Future <Output = AlwaysClean <S>> + Send + Unpin,
	S: ServiceHandle + Send + Unpin,
	S::Output: Send
{
	let mut service_handle = tokio::select!
	{
		_ = &mut shutdown => return AlwaysClean::new (()),
		constructor_result = constructor () =>
			constructor_result . into_value ()
	};

	state_channel . send_replace (ServiceState::Up);

	event_loop!
	{
		_ = &mut shutdown => ShouldTerminateClean::from (true),
		_ = &mut service_handle =>
		{
			state_channel . send_replace (ServiceState::Down);

			replace_down_service_with_shutdown
			(
				&mut shutdown,
				constructor (),
				&mut service_handle,
				&mut state_channel
			) . await
		},
		_ = sleep (replacement_interval) => replace_service_with_shutdown
		(
			&mut shutdown,
			constructor (),
			&mut service_handle,
			&mut state_channel
		) . await
	}
}
