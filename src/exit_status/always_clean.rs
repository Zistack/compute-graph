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

	pub fn split (self) -> (T, AlwaysClean <()>)
	{
		(self . value, AlwaysClean::default ())
	}

	pub fn with_value <U> (self, value: U) -> AlwaysClean <U>
	{
		AlwaysClean::new (value)
	}

	pub fn map_value <F, R> (self, f: F) -> AlwaysClean <R>
	where F: FnOnce (T) -> R
	{
		AlwaysClean::new (f (self . value))
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

impl From <()> for AlwaysClean
{
	fn from (unit: ()) -> Self
	{
		Self {value: unit}
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
