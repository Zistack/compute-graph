use std::future::Future;

use tokio::sync::{oneshot, watch};
use tokio::time::{Duration, sleep};

use crate::{service, select, event_loop};
use crate::exit_status::{ServiceExitStatus, AlwaysClean, ShouldTerminateClean};
use crate::service_handle::{ServiceHandle, CancellableServiceHandle};
use crate::service_state::ServiceState;
use crate::task_handle::TaskHandle;

use crate as compute_graph;

async fn replace_down_service <C, T>
(
	constructor_handle: C,
	service_handle: &mut CancellableServiceHandle <T>,
	state_channel: &mut watch::Sender <ServiceState>
)
where
	C: Future <Output = CancellableServiceHandle <T>>,
	T: Default + ServiceExitStatus + Unpin
{
	let new_service_handle = constructor_handle . await;

	let mut old_service_handle =
		std::mem::replace (service_handle, new_service_handle);

	state_channel . send_replace (ServiceState::Up);

	old_service_handle . shutdown ();
	old_service_handle . await;
}

async fn replace_service <C, T>
(
	mut constructor_handle: C,
	service_handle: &mut CancellableServiceHandle <T>,
	state_channel: &mut watch::Sender <ServiceState>
)
where
	C: Future <Output = CancellableServiceHandle <T>> + Unpin,
	T: Default + ServiceExitStatus + Unpin
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
		new_service_handle = &mut constructor_handle =>
		{
			let mut old_service_handle =
				std::mem::replace (service_handle, new_service_handle);

			// State UP (though we should still be UP at this point)

			old_service_handle . shutdown ();
			old_service_handle . await;
		}
	};
}

#[service]
pub async fn robust_service <F, C, T>
(
	mut constructor: F,
	mut state_channel: watch::Sender <ServiceState>
)
-> AlwaysClean
where
	F: FnMut () -> C,
	C: Future <Output = CancellableServiceHandle <T>> + Send,
	T: Default + ServiceExitStatus + Send + Unpin
{
	let mut service_handle = constructor () . await;

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
pub async fn robust_service_with_preemptive_replacement <F, C, T>
(
	mut constructor: F,
	mut state_channel: watch::Sender <ServiceState>,
	replacement_interval: Duration
)
-> AlwaysClean
where
	F: FnMut () -> C,
	C: Future <Output = CancellableServiceHandle <T>> + Send + Unpin,
	T: Default + ServiceExitStatus + Send + Unpin
{
	let mut service_handle = constructor () . await;

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
	C: TaskHandle + Future <Output = Option <S>> + Unpin,
	S: ServiceHandle + Unpin
{
	select!
	{
		_ = &mut *shutdown =>
		{
			constructor_handle . abort ();
			if let Some (mut new_service_handle) = constructor_handle . await
			{
				new_service_handle . shutdown ();
				new_service_handle . await;
			}

			ShouldTerminateClean::from (true)
		},
		constructor_result = &mut constructor_handle =>
		{
			// The constructor should always return Some (S) unless it was
			// aborted.
			let new_service_handle = constructor_result . unwrap ();
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
	C: TaskHandle + Future <Output = Option <S>> + Unpin,
	S: ServiceHandle + Unpin
{
	select!
	{
		_ = &mut *shutdown =>
		{
			constructor_handle . abort ();
			if let Some (mut new_service_handle) = constructor_handle . await
			{
				new_service_handle . shutdown ();
				new_service_handle . await;
			}

			ShouldTerminateClean::from (true)
		},
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
			// The constructor should always return Some (S) unless it was
			// aborted.
			let new_service_handle = constructor_result . unwrap ();
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
	C: TaskHandle + Future <Output = Option <S>> + Send + Unpin,
	S: ServiceHandle + Send + Unpin,
	S::Output: Send
{
	let mut constructor_handle = constructor ();

	let mut service_handle = tokio::select!
	{
		_ = &mut shutdown =>
		{
			constructor_handle . abort ();
			if let Some (mut service_handle) = constructor_handle . await
			{
				service_handle . shutdown ();
				service_handle . await;
			}

			return AlwaysClean::new (());
		},
		constructor_result = &mut constructor_handle =>
			constructor_result . unwrap ()
	};

	drop (constructor_handle);

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
	C: TaskHandle + Future <Output = Option <S>> + Send + Unpin,
	S: ServiceHandle + Send + Unpin,
	S::Output: Send
{
	let mut constructor_handle = constructor ();

	let mut service_handle = tokio::select!
	{
		_ = &mut shutdown =>
		{
			constructor_handle . abort ();
			if let Some (mut service_handle) = constructor_handle . await
			{
				service_handle . shutdown ();
				service_handle . await;
			}

			return AlwaysClean::new (());
		}
		constructor_result = &mut constructor_handle =>
			constructor_result . unwrap ()
	};

	drop (constructor_handle);

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
	};

	service_handle . shutdown ();
	service_handle . await;

	AlwaysClean::new (())
}
