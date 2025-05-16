#[macro_export]
macro_rules! __next
{
	($stream: ident -> $item: ident => $handler: expr) =>
	{
		match futures::StreamExt::next (&mut $stream) . await
		{
			std::option::Option::Some ($item) =>
				std::convert::Into::<$crate::exit_status::ShouldTerminateClean>::into ($handler),
			std::option::Option::None =>
				$crate::exit_status::ShouldTerminateClean::new ((), true)
		}
	};
	($stream: ident -> $item: ident ? => $handler: expr) =>
	{
		match futures::StreamExt::next (&mut $stream) . await
		{
			std::option::Option::Some ($item) =>
				std::convert::Into::<$crate::exit_status::ShouldTerminateWithStatus>::into ($handler),
			std::option::Option::None =>
				$crate::exit_status::ShouldTerminateWithStatus::new
				(
					(),
					std::option::Option::Some
					(
						$crate::exit_status::ExitStatus::Spurious
					)
				)
		}
	}
}
pub use __next as next;

#[macro_export]
macro_rules! __next_or_break
{
	($stream: ident -> $item: ident => $handler: expr) =>
	{
		match futures::StreamExt::next (&mut $stream) . await
		{
			std::option::Option::Some ($item) =>
			{
				let term_status =
					std::convert::Into::<$crate::exit_status::ShouldTerminateClean>::into ($handler);

				if term_status . should_terminate
				{
					break $crate::exit_status::AlwaysClean::new (());
				}
			},
			std::option::Option::None =>
				break $crate::exit_status::AlwaysClean::new (())
		}
	};
	($stream: ident -> $item: ident ? => $handler: expr) =>
	{
		match futures::StreamExt::next (&mut $stream) . await
		{
			std::option::Option::Some ($item) =>
			{
				let term_status =
					std::convert::Into::<$crate::exit_status::ShouldTerminateWithStatus>::into ($handler);

				if let Some (exit_status) term_satus . should_terminate_status
				{
					break $crate::exit_status::WithStatus::new ((), exit_status);
				}
			},
			std::option::Option::None =>
				break $crate::exit_status::WithStatus::new
				(
					(),
					$crate::exit_status::ExitStatus::Spurious
				)
		}
	}
}
pub use __next_or_break as next_or_break;

#[macro_export]
macro_rules! __next_or_return
{
	($stream: ident -> $item: ident => $handler: expr) =>
	{
		match futures::StreamExt::next (&mut $stream) . await
		{
			std::option::Option::Some ($item) =>
			{
				let term_status =
					std::convert::Into::<$crate::exit_status::ShouldTerminateClean>::into ($handler);

				if term_status . should_terminate
				{
					return $crate::exit_status::AlwaysClean::new (());
				}
			},
			std::option::Option::None =>
				return $crate::exit_status::AlwaysClean::new (())
		}
	};
	($stream: ident -> $item: ident ? => $handler: expr) =>
	{
		match futures::StreamExt::next (&mut $stream) . await
		{
			std::option::Option::Some ($item) =>
			{
				let term_status =
					std::convert::Into::<$crate::exit_status::ShouldTerminateWithStatus>::into ($handler);

				if let Some (exit_status) term_satus . should_terminate_status
				{
					return $crate::exit_status::WithStatus::new ((), exit_status);
				}
			},
			std::option::Option::None =>
				return $crate::exit_status::WithStatus::new
				(
					(),
					$crate::exit_status::ExitStatus::Spurious
				)
		}
	}
}
pub use __next_or_return as next_or_return;
