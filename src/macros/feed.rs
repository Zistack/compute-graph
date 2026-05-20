#[macro_export]
macro_rules! __feed
{
	($sink: ident, $item: expr) =>
	{
		$crate::handle_sink_result!
		(
			futures::SinkExt::feed (&mut $sink, $item) . await
		)
	};
	($sink: ident ?, $item: expr) =>
	{
		$crate::handle_sink_result!
		(
			?futures::SinkExt::feed (&mut $sink, $item) . await
		)
	}
}
pub use __feed as feed;
