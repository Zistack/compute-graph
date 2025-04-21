use std::future::Future;

use crate::exit_status::WithStatus;
use crate::service_handle::{ServiceHandle, CancellableServiceHandle};
use crate::task_handle::TaskHandle;

pub trait CancellableFallibleServiceFactory
{
	fn construct (&mut self)
	-> impl TaskHandle
		+ Future <Output = CancellableServiceHandle <WithStatus>>
		+ Unpin
		+ Send;
}

pub trait SignallableFallibleServiceFactory
{
	// Rare case of having to explain to the compiler that the returned service
	// should _not_ borrow from self.
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
		+ Unpin
		+ Send;
}
