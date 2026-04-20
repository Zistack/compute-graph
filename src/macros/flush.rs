#[macro_export]
macro_rules! __flush
{
	($sink: ident) =>
	{
		match futures::SinkExt::flush (&mut $sink) . await
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
	($sink: ident ?) =>
	{
		match futures::SinkExt::flush (&mut $sink) . await
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
pub use __flush as flush;

#[macro_export]
macro_rules! __flush_or_break
{
	($sink: ident) =>
	{
		if let std::result::Result::Err (sink_error) =
			futures::SinkExt::flush (&mut $sink) . await
		{
			tracing::event!
			(
				tracing::Level::WARN,
				error = %sink_error,
				"sink rejected item"
			);

			break std::convert::Into::into (());
		}
	};
	($sink: ident ?) =>
	{
		if let std::result::Result::Err (sink_error) =
			futures::SinkExt::flush (&mut $sink) . await
		{
			tracing::event!
			(
				tracing::Level::WARN,
				error = %sink_error,
				"sink rejected item"
			);

			break std::convert::Into::into
			(
				$crate::exit_status::ExitStatus::Spurious
			);
		}
	}
}
pub use __flush_or_break as flush_or_break;

#[macro_export]
macro_rules! __flush_or_return
{
	($sink: ident) =>
	{
		if let std::result::Result::Err (sink_error) =
			futures::SinkExt::flush (&mut $sink) . await
		{
			tracing::event!
			(
				tracing::Level::WARN,
				error = %sink_error,
				"sink rejected item"
			);

			return std::convert::Into::into (());
		}
	};
	($sink: ident ?) =>
	{
		if let std::result::Result::Err (sink_error) =
			futures::SinkExt::flush (&mut $sink) . await
		{
			tracing::event!
			(
				tracing::Level::WARN,
				error = %sink_error,
				"sink rejected item"
			);

			return std::convert::Into::into
			(
				$crate::exit_status::ExitStatus::Spurious
			);
		}
	}
}
pub use __flush_or_return as flush_or_return;
