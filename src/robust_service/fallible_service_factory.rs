use std::future::Future;

use crate::exit_status::ServiceExitStatus;
use crate::service_handle::{ServiceHandle, CancellableServiceHandle};
use crate::task_handle::TaskHandle;

pub trait CancellableFallibleServiceFactory
{
	fn construct (&mut self)
	-> impl TaskHandle
		+ Future
		<
			Output = CancellableServiceHandle
			<
				impl ServiceExitStatus + Default + Send + Unpin + 'static
			>
		>
		+ Send
		+ 'static;
}

pub trait SignallableFallibleServiceFactory
{
	fn construct (&mut self)
	-> impl TaskHandle
		+ Future
		<
			Output = Option
			<
				impl ServiceHandle
					+ Future <Output: ServiceExitStatus + Default + Send + Unpin + 'static>
					+ Unpin
					+ Send
					+ 'static
			>
		>
		+ Send;
}
