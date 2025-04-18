mod cancellable;
pub use cancellable::CancellableServiceHandle;

mod signallable;
pub use signallable::SignallableServiceHandle;

use std::future::Future;

use crate::exit_status::*;

pub trait ServiceHandle: Future
{
	fn shutdown (&mut self);

	async fn exit_status (&mut self) -> Option <ExitStatus>;

	fn take_output (&mut self) -> Option <Self::Output>;
}
