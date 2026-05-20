mod always_clean;
pub use always_clean::AlwaysClean;

mod with_status;
pub use with_status::WithStatus;

// Normally, we'd use the vocabulary 'clean' and 'dirty'.  Should I be doing
// that?
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
	pub fn into_result (self) -> Result <(), ()>
	{
		match self
		{
			ExitStatus::Clean => Ok (()),
			ExitStatus::Spurious => Err (())
		}
	}

	pub fn is_clean (&self) -> bool
	{
		match self
		{
			ExitStatus::Clean => true,
			ExitStatus::Spurious => false
		}
	}

	pub fn is_spurious (&self) -> bool
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

impl ServiceExitStatus for ()
{
	type Value = ();

	fn exit_status (&self) -> ExitStatus { ExitStatus::Clean }

	fn status_clean (&self) -> bool { true }

	fn status_spurious (&self) -> bool { false }
}

impl ServiceExitStatus for ExitStatus
{
	type Value = ();

	fn exit_status (&self) -> ExitStatus { *self }

	fn status_clean (&self) -> bool { self . is_clean () }

	fn status_spurious (&self) -> bool { self . is_spurious () }
}
