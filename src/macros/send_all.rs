#[macro_export]
macro_rules! __send_all
{
	($sink: ident, $stream: expr) =>
	{
		$crate::handle_sink_result!
		(
			futures::SinkExt::send_all (&mut $sink, $stream) . await
		)
	};
	($sink: ident ?, $stream: expr) =>
	{
		$crate::handle_sink_result!
		(
			?futures::SinkExt::send_all (&mut $sink, $stream) . await
		)
	}
}
pub use __send_all as send_all;
