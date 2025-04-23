use std::future::Future;

use crate::exit_status::WithStatus;
use crate::service_handle::{ServiceHandle, CancellableServiceHandle};
use crate::task_handle::TaskHandle;

pub trait CancellableFallibleServiceFactory
{
	fn construct (&mut self)
	-> impl TaskHandle
		+ Future <Output = CancellableServiceHandle <WithStatus>>
		+ Send;
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
					+ Future <Output = WithStatus>
					+ Unpin
					+ Send
					+ 'static
			>
		>
		+ Send;
}
