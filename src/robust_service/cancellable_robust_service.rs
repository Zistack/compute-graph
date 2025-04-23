use std::future::Future;
use std::pin::{Pin, pin};

use tokio::sync::watch;
use tokio::time::{Duration, sleep};

use crate::{service, select, event_loop};
use crate::exit_status::{ServiceExitStatus, AlwaysClean};
use crate::service_handle::{ServiceHandle, CancellableServiceHandle};
use crate::service_state::ServiceState;

use super::fallible_service_factory::CancellableFallibleServiceFactory;

use crate as compute_graph;

async fn replace_down_service <C, T>
(
	constructor_handle: Pin <&mut C>,
	service_handle: &mut CancellableServiceHandle <T>
)
where
	C: Future <Output = CancellableServiceHandle <T>>,
	T: Default + ServiceExitStatus + Unpin
{
	*service_handle = constructor_handle . await;
}

async fn replace_service <C, T>
(
	mut constructor_handle: Pin <&mut C>,
	service_handle: &mut CancellableServiceHandle <T>
)
where
	C: Future <Output = CancellableServiceHandle <T>>,
	T: Default + ServiceExitStatus + Unpin
{
	select!
	{
		_ = &mut *service_handle => replace_down_service
		(
			constructor_handle,
			service_handle
		) . await,
		new_service_handle = constructor_handle . as_mut () =>
		{
			let mut old_service_handle =
				std::mem::replace (service_handle, new_service_handle);

			old_service_handle . shutdown ();
			old_service_handle . await;
		}
	};
}

async fn replace_down_service_with_status_reporting <C, T>
(
	constructor_handle: Pin <&mut C>,
	service_handle: &mut CancellableServiceHandle <T>,
	status_sender: &mut watch::Sender <ServiceState>
)
where
	C: Future <Output = CancellableServiceHandle <T>>,
	T: Default + ServiceExitStatus + Unpin
{
	*service_handle = constructor_handle . await;

	status_sender . send_replace (ServiceState::Up);
}

async fn replace_service_with_status_reporting <C, T>
(
	mut constructor_handle: Pin <&mut C>,
	service_handle: &mut CancellableServiceHandle <T>,
	status_sender: &mut watch::Sender <ServiceState>
)
where
	C: Future <Output = CancellableServiceHandle <T>>,
	T: Default + ServiceExitStatus + Unpin
{
	select!
	{
		_ = &mut *service_handle => replace_down_service_with_status_reporting
		(
			constructor_handle,
			service_handle,
			status_sender
		) . await,
		new_service_handle = constructor_handle . as_mut () =>
		{
			let mut old_service_handle =
				std::mem::replace (service_handle, new_service_handle);

			// State UP (though we should still be UP at this point)

			old_service_handle . shutdown ();
			old_service_handle . await;
		}
	};
}

pub trait CancellableRobustService
{
	fn into_robust_service (self) -> CancellableServiceHandle <AlwaysClean>;

	fn into_robust_service_with_preemptive_replacement
	(
		self,
		replacement_interval: Duration
	)
	-> CancellableServiceHandle <AlwaysClean>;

	fn into_robust_service_with_status_reporting
	(
		self,
		status_sender: watch::Sender <ServiceState>
	)
	-> CancellableServiceHandle <AlwaysClean>;

	fn into_robust_service_with_preemptive_replacement_and_status_reporting
	(
		self,
		replacement_interval: Duration,
		status_sender: watch::Sender <ServiceState>
	)
	-> CancellableServiceHandle <AlwaysClean>;
}

impl <T> CancellableRobustService for T
where T: CancellableFallibleServiceFactory + Send + 'static
{
	#[service]
	async fn into_robust_service (mut self) -> AlwaysClean
	{
		let mut service_handle = self . construct () . await;

		event_loop!
		{
			_ = &mut service_handle => replace_down_service
			(
				pin! (self . construct ()),
				&mut service_handle
			) . await
		}
	}

	#[service]
	async fn into_robust_service_with_preemptive_replacement
	(
		mut self,
		replacement_interval: Duration
	)
	-> AlwaysClean
	{
		let mut service_handle = self . construct () . await;

		event_loop!
		{
			_ = &mut service_handle => replace_down_service
			(
				pin! (self . construct ()),
				&mut service_handle
			) . await,
			_ = sleep (replacement_interval) => replace_service
			(
				pin! (self . construct ()),
				&mut service_handle
			) . await
		}
	}

	#[service]
	async fn into_robust_service_with_status_reporting
	(
		mut self,
		mut status_sender: watch::Sender <ServiceState>
	)
	-> AlwaysClean
	{
		let mut service_handle = self . construct () . await;

		status_sender . send_replace (ServiceState::Up);

		event_loop!
		{
			_ = &mut service_handle =>
			{
				status_sender . send_replace (ServiceState::Down);

				replace_down_service_with_status_reporting
				(
					pin! (self . construct ()),
					&mut service_handle,
					&mut status_sender
				) . await
			}
		}
	}

	#[service]
	async fn into_robust_service_with_preemptive_replacement_and_status_reporting
	(
		mut self,
		replacement_interval: Duration,
		mut status_sender: watch::Sender <ServiceState>
	)
	-> AlwaysClean
	{
		let mut service_handle = self . construct () . await;

		status_sender . send_replace (ServiceState::Up);

		event_loop!
		{
			_ = &mut service_handle =>
			{
				status_sender . send_replace (ServiceState::Down);

				replace_down_service_with_status_reporting
				(
					pin! (self . construct ()),
					&mut service_handle,
					&mut status_sender
				) . await
			},
			_ = sleep (replacement_interval) => replace_service_with_status_reporting
			(
				pin! (self . construct ()),
				&mut service_handle,
				&mut status_sender
			) . await
		}
	}
}
