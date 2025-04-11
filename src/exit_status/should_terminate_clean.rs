use super::ServiceShouldTerminate;

pub struct ShouldTerminateClean <T = ()>
{
	pub (in crate::exit_status) value: T,
	pub should_terminate: bool
}

impl <T> ShouldTerminateClean <T>
{
	pub fn new (value: T, should_terminate: bool) -> Self
	{
		Self {value, should_terminate}
	}

	pub fn split (self) -> (T, ShouldTerminateClean <()>)
	{
		(self . value, ShouldTerminateClean::from (self . should_terminate))
	}

	pub fn with_value <U> (self, value: U) -> ShouldTerminateClean <U>
	{
		ShouldTerminateClean::new (value, self . should_terminate)
	}

	pub fn map_value <F, R> (self, f: F) -> ShouldTerminateClean <R>
	where F: FnOnce (T) -> R
	{
		ShouldTerminateClean::new (f (self . value), self . should_terminate)
	}
}

impl <T> Default for ShouldTerminateClean <T>
where T: Default
{
	fn default () -> Self
	{
		Self {value: T::default (), should_terminate: false}
	}
}

impl From <()> for ShouldTerminateClean
{
	fn from (unit: ()) -> Self
	{
		Self {value: unit, should_terminate: false}
	}
}

impl From <bool> for ShouldTerminateClean
{
	fn from (should_terminate: bool) -> Self
	{
		Self {value: (), should_terminate}
	}
}

impl <T> ServiceShouldTerminate for ShouldTerminateClean <T>
{
	fn should_terminate (&self) -> bool
	{
		self . should_terminate
	}
}
