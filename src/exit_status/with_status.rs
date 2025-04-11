use super::{ExitStatus, ServiceExitStatus, AlwaysClean};

pub struct WithStatus <T = ()>
{
	pub (in crate::exit_status) value: T,
	pub status: ExitStatus
}

impl <T> WithStatus <T>
{
	pub fn new (value: T, status: ExitStatus) -> Self
	{
		Self {value, status}
	}

	pub fn split (self) -> (T, WithStatus <()>)
	{
		(self . value, WithStatus::from (self . status))
	}

	pub fn with_value <U> (self, value: U) -> WithStatus <U>
	{
		WithStatus::new (value, self . status)
	}

	pub fn map_value <F, R> (self, f: F) -> WithStatus <R>
	where F: FnOnce (T) -> R
	{
		WithStatus::new (f (self . value), self . status)
	}

	pub fn into_value (self) -> T
	{
		self . value
	}
}

impl <T> Default for WithStatus <T>
where T: Default
{
	fn default () -> Self
	{
		Self {value: T::default (), status: ExitStatus::Clean}
	}
}

impl From <()> for WithStatus
{
	fn from (unit: ()) -> Self
	{
		Self {value: unit, status: ExitStatus::Clean}
	}
}

impl From <ExitStatus> for WithStatus
{
	fn from (status: ExitStatus) -> Self
	{
		Self {value: (), status}
	}
}

impl <T> From <AlwaysClean <T>> for WithStatus <T>
{
	fn from (always_clean: AlwaysClean <T>) -> Self
	{
		Self {value: always_clean . value, status: ExitStatus::Clean}
	}
}

impl <T> ServiceExitStatus for WithStatus <T>
{
	type Value = T;

	fn exit_status (&self) -> ExitStatus
	{
		self . status
	}
}
