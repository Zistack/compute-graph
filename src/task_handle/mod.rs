mod cancellable;
pub use cancellable::CancellableTaskHandle;

mod signallable;
pub use signallable::SignallableTaskHandle;

mod parallel_cancellable;
pub use parallel_cancellable::ParallelCancellableTaskHandle;

mod parallel_signallable;
pub use parallel_signallable::ParallelSignallableTaskHandle;

use std::future::Future;

pub trait TaskHandle: Future
{
	fn abort (&mut self);
}
