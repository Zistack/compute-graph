use super::{ExitStatus, ServiceExitStatus};

pub struct AlwaysClean <T = ()>
{
	pub (in crate::exit_status) value: T
}

impl <T> AlwaysClean <T>
{
	pub fn new (value: T) -> Self
	{
		Self {value}
	}

	pub fn into_value (self) -> T
	{
		self . value
	}
}

impl <T> Default for AlwaysClean <T>
where T: Default
{
	fn default () -> Self
	{
		Self {value: T::default ()}
	}
}

impl <T> ServiceExitStatus for AlwaysClean <T>
{
	type Value = T;

	fn exit_status (&self) -> ExitStatus
	{
		ExitStatus::Clean
	}
}
