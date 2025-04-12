#[macro_export]
macro_rules! __feed
{
	($sink: ident, $item: expr) =>
	{
		match <_ as futures::SinkExt <_>>::feed (&mut $sink, $item) . await
		{
			std::result::Result::Ok (()) =>
				$crate::exit_status::ShouldTerminateClean::new ((), false),
			std::result::Result::Err (sink_error) =>
			{
				tracing::event!
				(
					tracing::Level::WARN,
					error = %sink_error,
					"sink_rejected_item"
				);
				$crate::exit_status::ShouldTerminateClean::new ((), true)
			}
		}
	};
	($sink: ident ?, $item: expr) =>
	{
		match <_ as futures::SinkExt <_>>::feed (&mut $sink, $item) . await
		{
			std::result::Result::Ok (()) =>
				$crate::exit_status::ShouldTerminateWithStatus::new ((), None),
			std::result::Result::Err (sink_error) =>
			{
				tracing::event!
				(
					tracing::Level::ERROR,
					error = %sink_error,
					"sink rejected item"
				);
				$crate::exit_status::ShouldTerminateWithStatus::new
				(
					(),
					Some ($crate::exit_status::ExitStatus::Spurious)
				)
			}
		}
	}
}
pub use __feed as feed;

#[macro_export]
macro_rules! __next
{
	($stream: ident -> $item: ident => $handler: expr) =>
	{
		match <_ as futures::StreamExt>::next ($stream) . await
		{
			std::option::Option::Some ($item) =>
				<_ as std::convert::Into <_>>::into ($handler),
			std::option::Option::None =>
				$crate::exit_status::ShouldTerminateClean::new ((), true)
		}
	};
	($stream: ident -> $item: ident ? => $handler: expr) =>
	{
		match <_ as futures::StreamExt>::next ($stream) . await
		{
			std::option::Option::Some ($item) =>
				<_ as std::convert::Into <_>>::into ($handler),
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
macro_rules! __return_if_should_terminate
{
	($e: expr) =>
	{
		{
			let should_terminate = $e;
			if should_terminate . should_terminate ()
			{
				return <_ as std::convert::Into <_>>::into (should_terminate)
			}
		}
	}
}
pub use __return_if_should_terminate as return_if_should_terminate;
