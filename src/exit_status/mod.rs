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

impl ExitStatus
{
	fn into_result (self) -> Result <(), ()>
	{
		match self
		{
			ExitStatus::Clean => Ok (()),
			ExitStatus::Spurious => Err (())
		}
	}

	fn is_clean (&self) -> bool
	{
		match self
		{
			ExitStatus::Clean => true,
			ExitStatus::Spurious => false
		}
	}

	fn is_spurious (&self) -> bool
	{
		match self
		{
			ExitStatus::Clean => false,
			ExitStatus::Spurious => true
		}
	}
}

pub trait ServiceExitStatus
{
	type Value;

	fn exit_status (&self) -> ExitStatus;

	fn status_clean (&self) -> bool
	{
		self . exit_status () . is_clean ()
	}

	fn status_spurious (&self) -> bool
	{
		self . exit_status () . is_spurious ()
	}
}

pub trait ServiceShouldTerminate
{
	fn should_terminate (&self) -> bool;
}
