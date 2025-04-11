use super::{ExitStatus, ServiceShouldTerminate, ShouldTerminateClean};

pub struct ShouldTerminateWithStatus <T = ()>
{
	pub (in crate::exit_status) value: T,
	pub should_terminate_status: Option <ExitStatus>
}

impl <T> ShouldTerminateWithStatus <T>
{
	pub fn new (value: T, should_terminate_status: Option <ExitStatus>) -> Self
	{
		Self {value, should_terminate_status}
	}

	pub fn split (self) -> (T, ShouldTerminateWithStatus <()>)
	{
		(
			self . value,
			ShouldTerminateWithStatus::from (self . should_terminate_status)
		)
	}

	pub fn with_value <U> (self, value: U) -> ShouldTerminateWithStatus <U>
	{
		ShouldTerminateWithStatus::new (value, self . should_terminate_status)
	}

	pub fn map_value <F, R> (self, f: F) -> ShouldTerminateWithStatus <R>
	where F: FnOnce (T) -> R
	{
		ShouldTerminateWithStatus::new
		(
			f (self . value),
			self . should_terminate_status
		)
	}
}

impl <T> Default for ShouldTerminateWithStatus <T>
where T: Default
{
	fn default () -> Self
	{
		Self {value: T::default (), should_terminate_status: None}
	}
}

impl From <()> for ShouldTerminateWithStatus
{
	fn from (unit: ()) -> Self
	{
		Self {value: unit, should_terminate_status: None}
	}
}

impl From <ExitStatus> for ShouldTerminateWithStatus
{
	fn from (status: ExitStatus) -> Self
	{
		Self {value: (), should_terminate_status: Some (status)}
	}
}

impl From <Option <ExitStatus>> for ShouldTerminateWithStatus
{
	fn from (should_terminate_status: Option <ExitStatus>) -> Self
	{
		Self {value: (), should_terminate_status}
	}
}

impl <T> From <ShouldTerminateClean <T>> for ShouldTerminateWithStatus <T>
{
	fn from (should_terminate_clean: ShouldTerminateClean <T>) -> Self
	{
		Self
		{
			value: should_terminate_clean . value,
			should_terminate_status: should_terminate_clean
				. should_terminate
				. then_some (ExitStatus::Clean)
		}
	}
}

impl <T> ServiceShouldTerminate for ShouldTerminateWithStatus <T>
{
	fn should_terminate (&self) -> bool
	{
		self . should_terminate_status . is_some ()
	}
}
