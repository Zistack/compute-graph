#[macro_export]
macro_rules! __flush
{
	($sink: ident) =>
	{
		$crate::handle_sink_result!
		(
			futures::SinkExt::flush (&mut $sink) . await
		)
	};
	($sink: ident ?) =>
	{
		$crate::handle_sink_result!
		(
			?futures::SinkExt::flush (&mut $sink) . await
		)
	}
}
pub use __flush as flush;
