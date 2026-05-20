use super::{ExitStatus, ServiceExitStatus};

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

impl <T> ServiceExitStatus for WithStatus <T>
{
	type Value = T;

	fn exit_status (&self) -> ExitStatus
	{
		self . status
	}
}
