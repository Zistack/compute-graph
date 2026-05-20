use std::future::Future;
use std::ops::ControlFlow;
use std::pin::{Pin, pin};

use tokio::sync::{oneshot, watch};
use tokio::time::{Duration, sleep};

use crate::{service, select, event_loop, check_break};
use crate::service_handle::{ServiceHandle, SignallableServiceHandle};
use crate::service_state::ServiceState;
use crate::task_handle::TaskHandle;

use super::fallible_service_factory::SignallableFallibleServiceFactory;

use crate as compute_graph;

async fn shutdown_constructor_handle <C, S>
(
	mut constructor_handle: Pin <&mut C>
)
where
	C: TaskHandle + Future <Output = Option <S>>,
	S: ServiceHandle
{
	constructor_handle . as_mut () . abort ();

	if let Some (mut new_service_handle) = constructor_handle . await
	{
		new_service_handle . shutdown ();
		new_service_handle . await;
	}
}

async fn replace_down_service <C, S>
(
	shutdown: &mut oneshot::Receiver <()>,
	mut constructor_handle: Pin <&mut C>,
	service_handle: &mut S
)
-> ControlFlow <()>
where
	C: TaskHandle + Future <Output = Option <S>>,
	S: ServiceHandle
{
	tokio::select!
	{
		biased;
		_ = &mut *shutdown =>
		{
			shutdown_constructor_handle (constructor_handle) . await;
			ControlFlow::Break (())
		},
		constructor_result = constructor_handle . as_mut () =>
		{
			*service_handle = constructor_result . unwrap ();
			ControlFlow::Continue (())
		}
	}
}

async fn replace_service <C, S>
(
	shutdown: &mut oneshot::Receiver <()>,
	mut constructor_handle: Pin <&mut C>,
	service_handle: &mut S
)
-> ControlFlow <()>
where
	C: TaskHandle + Future <Output = Option <S>>,
	S: ServiceHandle + Unpin
{
	select!
	{
		_ = &mut *shutdown =>
		{
			shutdown_constructor_handle (constructor_handle) . await;
			ControlFlow::Break (())
		},
		_ = &mut *service_handle => replace_down_service
		(
			shutdown,
			constructor_handle,
			service_handle
		) . await,
		constructor_result = constructor_handle . as_mut () =>
		{
			// The constructor should always return Some (S) unless it was
			// aborted.
			let new_service_handle = constructor_result . unwrap ();
			let mut old_service_handle =
				std::mem::replace (service_handle, new_service_handle);

			old_service_handle . shutdown ();
			old_service_handle . await;

			ControlFlow::Continue (())
		}
	}
}

pub async fn replace_down_service_with_status_reporting <C, S>
(
	shutdown: &mut oneshot::Receiver <()>,
	mut constructor_handle: Pin <&mut C>,
	service_handle: &mut S,
	status_sender: &mut watch::Sender <ServiceState>
)
-> ControlFlow <()>
where
	C: TaskHandle + Future <Output = Option <S>>,
	S: ServiceHandle
{
	tokio::select!
	{
		biased;
		_ = &mut *shutdown =>
		{
			shutdown_constructor_handle (constructor_handle) . await;
			ControlFlow::Break (())
		},
		constructor_result = constructor_handle . as_mut () =>
		{
			// The constructor should always return Some (S) unless it was
			// aborted.
			*service_handle = constructor_result . unwrap ();

			status_sender . send_replace (ServiceState::Up);

			ControlFlow::Continue (())
		}
	}
}

pub async fn replace_service_with_status_reporting <C, S>
(
	shutdown: &mut oneshot::Receiver <()>,
	mut constructor_handle: Pin <&mut C>,
	service_handle: &mut S,
	status_sender: &mut watch::Sender <ServiceState>
)
-> ControlFlow <()>
where
	C: TaskHandle + Future <Output = Option <S>>,
	S: ServiceHandle + Unpin
{
	tokio::select!
	{
		biased;
		_ = &mut *shutdown =>
		{
			shutdown_constructor_handle (constructor_handle) . await;
			ControlFlow::Break (())
		},
		_ = &mut *service_handle =>
		{
			status_sender . send_replace (ServiceState::Down);

			replace_down_service_with_status_reporting
			(
				shutdown,
				constructor_handle,
				service_handle,
				status_sender
			) . await
		},
		constructor_result = constructor_handle . as_mut () =>
		{
			// The constructor should always return Some (S) unless it was
			// aborted.
			let new_service_handle = constructor_result . unwrap ();
			let mut old_service_handle =
				std::mem::replace (service_handle, new_service_handle);

			// State UP (though we should still be UP at this point)

			old_service_handle . shutdown ();
			old_service_handle . await;

			ControlFlow::Continue (())
		}
	}
}

pub trait SignallableRobustService
{
	fn into_robust_service (self) -> SignallableServiceHandle <()>;

	fn into_robust_service_with_preemptive_replacement
	(
		self,
		replacement_interval: Duration
	)
	-> SignallableServiceHandle <()>;

	fn into_robust_service_with_status_reporting
	(
		self,
		status_sender: watch::Sender <ServiceState>
	)
	-> SignallableServiceHandle <()>;

	fn into_robust_service_with_preemptive_replacement_and_status_reporting
	(
		self,
		replacement_interval: Duration,
		status_sender: watch::Sender <ServiceState>
	)
	-> SignallableServiceHandle <()>;
}

macro_rules! init_service_handle_with_shutdown
{
	($constructor_handle: expr, $shutdown: expr) =>
	{{
		let mut constructor_handle = $constructor_handle;
		let shutdown = $shutdown;

		tokio::select!
		{
			biased;
			_ = shutdown =>
			{
				constructor_handle . as_mut () . abort ();
				if let Some (mut service_handle) = constructor_handle . await
				{
					service_handle . shutdown ();
					service_handle . await;
				}

				return;
			},
			constructor_result = &mut constructor_handle =>
				constructor_result . unwrap ()
		}
	}}
}

impl <T> SignallableRobustService for T
where T: SignallableFallibleServiceFactory + Send + 'static
{
	#[service (shutdown = shutdown)]
	async fn into_robust_service (mut self)
	{
		let mut service_handle = init_service_handle_with_shutdown!
		(
			pin! (self . construct ()),
			&mut shutdown
		);

		event_loop!
		{
			?&mut shutdown,
			_ = &mut service_handle => check_break!
			(
				replace_down_service
				(
					&mut shutdown,
					pin! (self . construct ()),
					&mut service_handle
				) . await
			)
		};

		service_handle . shutdown ();
		service_handle . await;
	}

	#[service (shutdown = shutdown)]
	async fn into_robust_service_with_preemptive_replacement
	(
		mut self,
		replacement_interval: Duration
	)
	{
		let mut service_handle = init_service_handle_with_shutdown!
		(
			pin! (self . construct ()),
			&mut shutdown
		);

		event_loop!
		{
			?&mut shutdown,
			_ = &mut service_handle => check_break!
			(
				replace_down_service
				(
					&mut shutdown,
					pin! (self . construct ()),
					&mut service_handle
				) . await
			),
			_ = sleep (replacement_interval) => check_break!
			(
				replace_service
				(
					&mut shutdown,
					pin! (self . construct ()),
					&mut service_handle
				) . await
			)
		};

		service_handle . shutdown ();
		service_handle . await;
	}

	#[service (shutdown = shutdown)]
	async fn into_robust_service_with_status_reporting
	(
		mut self,
		mut status_sender: watch::Sender <ServiceState>
	)
	{
		let mut service_handle = init_service_handle_with_shutdown!
		(
			pin! (self . construct ()),
			&mut shutdown
		);

		status_sender . send_replace (ServiceState::Up);

		event_loop!
		{
			?&mut shutdown,
			_ = &mut service_handle =>
			{
				status_sender . send_replace (ServiceState::Down);

				check_break!
				(
					replace_down_service_with_status_reporting
					(
						&mut shutdown,
						pin! (self . construct ()),
						&mut service_handle,
						&mut status_sender
					) . await
				)
			}
		};

		service_handle . shutdown ();
		service_handle . await;
	}

	#[service (shutdown = shutdown)]
	async fn into_robust_service_with_preemptive_replacement_and_status_reporting
	(
		mut self,
		replacement_interval: Duration,
		mut status_sender: watch::Sender <ServiceState>
	)
	{
		let mut service_handle = init_service_handle_with_shutdown!
		(
			pin! (self . construct ()),
			&mut shutdown
		);

		status_sender . send_replace (ServiceState::Up);

		event_loop!
		{
			?&mut shutdown,
			_ = &mut service_handle =>
			{
				status_sender . send_replace (ServiceState::Down);

				check_break!
				(
					replace_down_service_with_status_reporting
					(
						&mut shutdown,
						pin! (self . construct ()),
						&mut service_handle,
						&mut status_sender
					) . await
				)
			},
			_ = sleep (replacement_interval) =>	check_break!
			(
				replace_service_with_status_reporting
				(
					&mut shutdown,
					pin! (self . construct ()),
					&mut service_handle,
					&mut status_sender
				) . await
			)
		};

		service_handle . shutdown ();
		service_handle . await;
	}
}
