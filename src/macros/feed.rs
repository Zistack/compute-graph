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
					"sink rejected item"
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
macro_rules! __feed_or_break
{
	($sink: ident, $item: expr) =>
	{
		if let std::result::Result::Err (sink_error) =
			<_ as futures::SinkExt <_>>::feed (&mut $sink, $item) . await
		{
			tracing::event!
			(
				tracing::Level::WARN,
				error = %sink_error,
				"sink rejected item"
			);

			break <() as std::convert::Into <_>>::into (());
		}
	};
	($sink: ident ?, $item: expr) =>
	{
		if let std::result::Result::Err (sink_error) =
			<_ as futures::SinkExt <_>>::feed (&mut $sink, $item) . await
		{
			tracing::event!
			(
				tracing::Level::WARN,
				error = %sink_error,
				"sink rejected item"
			);

			break <$crate::exit_status::ExitStatus as std::convert::Into <_>>::into
			(
				$crate::exit_status::ExitStatus::Spurious
			);
		}
	}
}
pub use __feed_or_break as feed_or_break;

#[macro_export]
macro_rules! __feed_or_return
{
	($sink: ident, $item: expr) =>
	{
		if let std::result::Result::Err (sink_error) =
			<_ as futures::SinkExt <_>>::feed (&mut $sink, $item) . await
		{
			tracing::event!
			(
				tracing::Level::WARN,
				error = %sink_error,
				"sink rejected item"
			);

			return <() as std::convert::Into <_>>::into (());
		}
	};
	($sink: ident ?, $item: expr) =>
	{
		if let std::result::Result::Err (sink_error) =
			<_ as futures::SinkExt <_>>::feed (&mut $sink, $item) . await
		{
			tracing::event!
			(
				tracing::Level::WARN,
				error = %sink_error,
				"sink rejected item"
			);

			return <$crate::exit_status::ExitStatus as std::convert::Into <_>>::into
			(
				$crate::exit_status::ExitStatus::Spurious
			);
		}
	}
}
pub use __feed_or_return as feed_or_return;
