mod cancellable;
pub use cancellable::CancellableTaskHandle;

mod signallable;
pub use signallable::SignallableTaskHandle;

use std::future::Future;

pub trait TaskHandle: Future
{
	fn abort (&mut self);
}
