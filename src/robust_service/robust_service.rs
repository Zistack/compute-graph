use std::future::Future;

use tokio::sync::watch;
use tokio::time::{Duration, sleep};

use crate::{service, event_loop};
use crate::exit_status::{ServiceExitStatus, AlwaysClean, ShouldTerminateClean};
use crate::service_handle::{ServiceHandle, CancellableServiceHandle};
use crate::service_state::ServiceState;
use crate::task_handle::TaskHandle;

use super::replace_service::*;

use crate as compute_graph;

macro_rules! init_service_handle_with_shutdown
{
	($constructor: expr, $shutdown: expr) =>
	{
		{
			let mut constructor_handle = $constructor ();

			tokio::select!
			{
				_ = &mut $shutdown =>
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
			}
		}
	}
}

#[service]
pub async fn robust_service <F, C, T> (mut constructor: F) -> AlwaysClean
where
	F: FnMut () -> C,
	C: Future <Output = CancellableServiceHandle <T>> + Send,
	T: Default + ServiceExitStatus + Send + Unpin
{
	let mut service_handle = constructor () . await;

	event_loop!
	{
		_ = &mut service_handle => replace_down_service
		(
			constructor (),
			&mut service_handle
		) . await
	}
}

#[service]
pub async fn robust_service_with_preemptive_replacement <F, C, T>
(
	mut constructor: F,
	replacement_interval: Duration
)
-> AlwaysClean
where
	F: FnMut () -> C,
	C: Future <Output = CancellableServiceHandle <T>> + Send + Unpin,
	T: Default + ServiceExitStatus + Send + Unpin
{
	let mut service_handle = constructor () . await;

	event_loop!
	{
		_ = &mut service_handle => replace_down_service
		(
			constructor (),
			&mut service_handle
		) . await,
		_ = sleep (replacement_interval) => replace_service
		(
			constructor (),
			&mut service_handle
		) . await
	}
}

#[service (shutdown = shutdown)]
pub async fn robust_service_with_shutdown <F, C, S>
(
	mut constructor: F
)
-> AlwaysClean
where
	F: FnMut () -> C,
	C: TaskHandle + Future <Output = Option <S>> + Send + Unpin,
	S: ServiceHandle + Send + Unpin,
	S::Output: Send
{
	let mut service_handle =
		init_service_handle_with_shutdown! (constructor, shutdown);

	event_loop!
	{
		_ = &mut shutdown => ShouldTerminateClean::from (true),
		_ = &mut service_handle => replace_down_service_with_shutdown
		(
			&mut shutdown,
			constructor (),
			&mut service_handle
		) . await
	};

	service_handle . shutdown ();
	service_handle . await;

	AlwaysClean::new (())
}

#[service (shutdown = shutdown)]
pub async fn robust_service_with_shutdown_and_preemptive_replacement <F, C, S>
(
	mut constructor: F,
	replacement_interval: Duration
)
-> AlwaysClean
where
	F: FnMut () -> C,
	C: TaskHandle + Future <Output = Option <S>> + Send + Unpin,
	S: ServiceHandle + Send + Unpin,
	S::Output: Send
{
	let mut service_handle =
		init_service_handle_with_shutdown! (constructor, shutdown);

	event_loop!
	{
		_ = &mut shutdown => ShouldTerminateClean::from (true),
		_ = &mut service_handle => replace_down_service_with_shutdown
		(
			&mut shutdown,
			constructor (),
			&mut service_handle
		) . await,
		_ = sleep (replacement_interval) => replace_service_with_shutdown
		(
			&mut shutdown,
			constructor (),
			&mut service_handle
		) . await
	};

	service_handle . shutdown ();
	service_handle . await;

	AlwaysClean::new (())
}

#[service]
pub async fn robust_service_with_report <F, C, T>
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

			replace_down_service_with_report
			(
				constructor (),
				&mut service_handle,
				&mut state_channel
			) . await
		}
	}
}

#[service]
pub async fn robust_service_with_preemptive_replacement_and_report <F, C, T>
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

			replace_down_service_with_report
			(
				constructor (),
				&mut service_handle,
				&mut state_channel
			) . await
		},
		_ = sleep (replacement_interval) => replace_service_with_report
		(
			constructor (),
			&mut service_handle,
			&mut state_channel
		) . await
	}
}

#[service (shutdown = shutdown)]
pub async fn robust_service_with_shutdown_and_report <F, C, S>
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
	let mut service_handle =
		init_service_handle_with_shutdown! (constructor, shutdown);

	state_channel . send_replace (ServiceState::Up);

	event_loop!
	{
		_ = &mut shutdown => ShouldTerminateClean::from (true),
		_ = &mut service_handle =>
		{
			state_channel . send_replace (ServiceState::Down);

			replace_down_service_with_shutdown_and_report
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
pub async fn robust_service_with_shutdown_and_preemptive_replacement_and_report <F, C, S>
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
	let mut service_handle =
		init_service_handle_with_shutdown! (constructor, shutdown);

	state_channel . send_replace (ServiceState::Up);

	event_loop!
	{
		_ = &mut shutdown => ShouldTerminateClean::from (true),
		_ = &mut service_handle =>
		{
			state_channel . send_replace (ServiceState::Down);

			replace_down_service_with_shutdown_and_report
			(
				&mut shutdown,
				constructor (),
				&mut service_handle,
				&mut state_channel
			) . await
		},
		_ = sleep (replacement_interval) =>
			replace_service_with_shutdown_and_report
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
