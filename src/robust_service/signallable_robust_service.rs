use std::future::Future;

use tokio::sync::{oneshot, watch};
use tokio::time::{Duration, sleep};

use crate::{service, select, event_loop};
use crate::exit_status::{AlwaysClean, ShouldTerminateClean};
use crate::service_handle::{ServiceHandle, SignallableServiceHandle};
use crate::service_state::ServiceState;
use crate::task_handle::TaskHandle;

use super::fallible_service_factory::SignallableFallibleServiceFactory;

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

async fn replace_down_service <C, S>
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

async fn replace_service <C, S>
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
		_ = &mut *service_handle => replace_down_service
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

pub async fn replace_down_service_with_status_reporting <C, S>
(
	shutdown: &mut oneshot::Receiver <()>,
	mut constructor_handle: C,
	service_handle: &mut S,
	status_sender: &mut watch::Sender <ServiceState>
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

			status_sender . send_replace (ServiceState::Up);
		}
	}
}

pub async fn replace_service_with_status_reporting <C, S>
(
	shutdown: &mut oneshot::Receiver <()>,
	mut constructor_handle: C,
	service_handle: &mut S,
	status_sender: &mut watch::Sender <ServiceState>
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
			status_sender . send_replace (ServiceState::Down);

			replace_down_service_with_status_reporting
			(
				shutdown,
				constructor_handle,
				service_handle,
				status_sender
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

pub trait SignallableRobustService
{
	fn into_robust_service (self) -> SignallableServiceHandle <AlwaysClean>;

	fn into_robust_service_with_preemptive_replacement
	(
		self,
		replacement_interval: Duration
	)
	-> SignallableServiceHandle <AlwaysClean>;

	fn into_robust_service_with_status_reporting
	(
		self,
		status_sender: watch::Sender <ServiceState>
	)
	-> SignallableServiceHandle <AlwaysClean>;

	fn into_robust_service_with_preemptive_replacement_and_status_reporting
	(
		self,
		replacement_interval: Duration,
		status_sender: watch::Sender <ServiceState>
	)
	-> SignallableServiceHandle <AlwaysClean>;
}

macro_rules! init_service_handle_with_shutdown
{
	($constructor_handle: expr, $shutdown: expr) =>
	{
		{
			let mut constructor_handle = $constructor_handle;

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

impl <T> SignallableRobustService for T
where T: SignallableFallibleServiceFactory + Send + 'static
{
	#[service (shutdown = shutdown)]
	async fn into_robust_service (mut self) -> AlwaysClean
	{
		let mut service_handle = init_service_handle_with_shutdown!
		(
			self . construct (),
			shutdown
		);

		event_loop!
		{
			_ = &mut shutdown => ShouldTerminateClean::from (true),
			_ = &mut service_handle => replace_down_service
			(
				&mut shutdown,
				self . construct (),
				&mut service_handle
			) . await
		};

		service_handle . shutdown ();
		service_handle . await;

		AlwaysClean::new (())
	}

	#[service (shutdown = shutdown)]
	async fn into_robust_service_with_preemptive_replacement
	(
		mut self,
		replacement_interval: Duration
	)
	-> AlwaysClean
	{
		let mut service_handle = init_service_handle_with_shutdown!
		(
			self . construct (),
			shutdown
		);

		event_loop!
		{
			_ = &mut shutdown => ShouldTerminateClean::from (true),
			_ = &mut service_handle => replace_down_service
			(
				&mut shutdown,
				self . construct (),
				&mut service_handle
			) . await,
			_ = sleep (replacement_interval) => replace_service
			(
				&mut shutdown,
				self . construct (),
				&mut service_handle
			) . await
		};

		service_handle . shutdown ();
		service_handle . await;

		AlwaysClean::new (())
	}

	#[service (shutdown = shutdown)]
	async fn into_robust_service_with_status_reporting
	(
		mut self,
		mut status_sender: watch::Sender <ServiceState>
	)
	-> AlwaysClean
	{
		let mut service_handle = init_service_handle_with_shutdown!
		(
			self . construct (),
			shutdown
		);

		status_sender . send_replace (ServiceState::Up);

		event_loop!
		{
			_ = &mut shutdown => ShouldTerminateClean::from (true),
			_ = &mut service_handle =>
			{
				status_sender . send_replace (ServiceState::Down);

				replace_down_service_with_status_reporting
				(
					&mut shutdown,
					self . construct (),
					&mut service_handle,
					&mut status_sender
				) . await
			}
		};

		service_handle . shutdown ();
		service_handle . await;

		AlwaysClean::new (())
	}

	#[service (shutdown = shutdown)]
	async fn into_robust_service_with_preemptive_replacement_and_status_reporting
	(
		mut self,
		replacement_interval: Duration,
		mut status_sender: watch::Sender <ServiceState>
	)
	-> AlwaysClean
	{
		let mut service_handle = init_service_handle_with_shutdown!
		(
			self . construct (),
			shutdown
		);

		status_sender . send_replace (ServiceState::Up);

		event_loop!
		{
			_ = &mut shutdown => ShouldTerminateClean::from (true),
			_ = &mut service_handle =>
			{
				status_sender . send_replace (ServiceState::Down);

				replace_down_service_with_status_reporting
				(
					&mut shutdown,
					self . construct (),
					&mut service_handle,
					&mut status_sender
				) . await
			},
			_ = sleep (replacement_interval) =>
				replace_service_with_status_reporting
			(
				&mut shutdown,
				self . construct (),
				&mut service_handle,
				&mut status_sender
			) . await
		};

		service_handle . shutdown ();
		service_handle . await;

		AlwaysClean::new (())
	}
}
