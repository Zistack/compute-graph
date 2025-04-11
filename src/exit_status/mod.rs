mod always_clean;
pub use always_clean::AlwaysClean;

mod with_status;
pub use with_status::WithStatus;

mod should_terminate_clean;
pub use should_terminate_clean::ShouldTerminateClean;

mod should_terminate_with_status;
pub use should_terminate_with_status::ShouldTerminateWithStatus;

#[derive (Copy, Clone, Hash, PartialEq, Eq)]
pub enum ExitStatus
{
	Clean,
	Spurious
}

impl Default for ExitStatus
{
	fn default () -> Self
	{
		Self::Clean
	}
}

pub trait ServiceExitStatus
{
	type Value;

	fn exit_status (&self) -> ExitStatus;
}

pub trait ServiceShouldTerminate
{
	fn should_terminate (&self) -> bool;
}
