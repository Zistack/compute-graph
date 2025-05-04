#[macro_export]
macro_rules! __next
{
	($stream: ident -> $item: ident => $handler: expr) =>
	{
		match <_ as futures::StreamExt>::next ($stream) . await
		{
			std::option::Option::Some ($item) =>
				<_ as std::convert::Into <$crate::exit_status::ShouldTerminateClean>>::into ($handler),
			std::option::Option::None =>
				$crate::exit_status::ShouldTerminateClean::new ((), true)
		}
	};
	($stream: ident -> $item: ident ? => $handler: expr) =>
	{
		match <_ as futures::StreamExt>::next ($stream) . await
		{
			std::option::Option::Some ($item) =>
				<_ as std::convert::Into <$crate::exit_status::ShouldTerminateWithStatus>>::into ($handler),
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
