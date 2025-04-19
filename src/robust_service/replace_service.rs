use std::future::Future;

use tokio::sync::{oneshot, watch};

use crate::select;
use crate::exit_status::{ServiceExitStatus, ShouldTerminateClean};
use crate::service_handle::{ServiceHandle, CancellableServiceHandle};
use crate::service_state::ServiceState;
use crate::task_handle::TaskHandle;

use crate as compute_graph;

macro_rules! shutdown_constructor_handle
{
	($constructor_handle: expr) =>
	{
		{
			$constructor_handle . abort ();

			if let Some (mut new_service_handle) = $constructor_handle . await
			{
				new_service_handle . shutdown ();
				new_service_handle . await;
			}

			ShouldTerminateClean::from (true)
		}
	}
}

pub async fn replace_down_service <C, T>
(
	constructor_handle: C,
	service_handle: &mut CancellableServiceHandle <T>
)
where
	C: Future <Output = CancellableServiceHandle <T>>,
	T: Default + ServiceExitStatus + Unpin
{
	*service_handle = constructor_handle . await;
}

pub async fn replace_service <C, T>
(
	mut constructor_handle: C,
	service_handle: &mut CancellableServiceHandle <T>
)
where
	C: Future <Output = CancellableServiceHandle <T>> + Unpin,
	T: Default + ServiceExitStatus + Unpin
{
	select!
	{
		_ = &mut *service_handle => replace_down_service
		(
			constructor_handle,
			service_handle
		) . await,
		new_service_handle = &mut constructor_handle =>
		{
			let mut old_service_handle =
				std::mem::replace (service_handle, new_service_handle);

			old_service_handle . shutdown ();
			old_service_handle . await;
		}
	};
}

pub async fn replace_down_service_with_shutdown <C, S>
(
	shutdown: &mut oneshot::Receiver <()>,
	mut constructor_handle: C,
	service_handle: &mut S
)
-> ShouldTerminateClean
where
	C: TaskHandle + Future <Output = Option <S>> + Unpin,
	S: ServiceHandle + Unpin
{
	select!
	{
		_ = &mut *shutdown => shutdown_constructor_handle! (constructor_handle),
		constructor_result = &mut constructor_handle =>
			*service_handle = constructor_result . unwrap ()
	}
}

pub async fn replace_service_with_shutdown <C, S>
(
	shutdown: &mut oneshot::Receiver <()>,
	mut constructor_handle: C,
	service_handle: &mut S
)
-> ShouldTerminateClean
where
	C: TaskHandle + Future <Output = Option <S>> + Unpin,
	S: ServiceHandle + Unpin
{
	select!
	{
		_ = &mut *shutdown => shutdown_constructor_handle! (constructor_handle),
		_ = &mut *service_handle => replace_down_service_with_shutdown
		(
			shutdown,
			constructor_handle,
			service_handle
		) . await,
		constructor_result = &mut constructor_handle =>
		{
			// The constructor should always return Some (S) unless it was
			// aborted.
			let new_service_handle = constructor_result . unwrap ();
			let mut old_service_handle =
				std::mem::replace (service_handle, new_service_handle);

			old_service_handle . shutdown ();
			old_service_handle . await;
		}
	}
}

pub async fn replace_down_service_with_report <C, T>
(
	constructor_handle: C,
	service_handle: &mut CancellableServiceHandle <T>,
	state_channel: &mut watch::Sender <ServiceState>
)
where
	C: Future <Output = CancellableServiceHandle <T>>,
	T: Default + ServiceExitStatus + Unpin
{
	*service_handle = constructor_handle . await;

	state_channel . send_replace (ServiceState::Up);
}

pub async fn replace_service_with_report <C, T>
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
		_ = &mut *service_handle => replace_down_service_with_report
		(
			constructor_handle,
			service_handle,
			state_channel
		) . await,
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

pub async fn replace_down_service_with_shutdown_and_report <C, S>
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
		_ = &mut *shutdown => shutdown_constructor_handle! (constructor_handle),
		constructor_result = &mut constructor_handle =>
		{
			// The constructor should always return Some (S) unless it was
			// aborted.
			*service_handle = constructor_result . unwrap ();

			state_channel . send_replace (ServiceState::Up);
		}
	}
}

pub async fn replace_service_with_shutdown_and_report <C, S>
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
		_ = &mut *shutdown => shutdown_constructor_handle! (constructor_handle),
		_ = &mut *service_handle =>
		{
			state_channel . send_replace (ServiceState::Down);

			replace_down_service_with_shutdown_and_report
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
