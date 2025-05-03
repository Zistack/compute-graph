use bytes::Bytes;
use tracing::{Level, event};
use tungstenite::{Message, Utf8Bytes};

pub trait InputFormat
{
	type Intermediate;

	fn convert (i: Self::Intermediate) -> Option <Message>;
}

pub trait OutputFormat
{
	type External;

	fn convert_text (utf8_bytes: Utf8Bytes) -> Option <Self::External>;

	fn convert_binary (bytes: Bytes) -> Option <Self::External>;
}

#[derive (Copy, Clone, Debug)]
pub struct Text;

impl InputFormat for Text
{
	type Intermediate = Utf8Bytes;

	fn convert (i: Self::Intermediate) -> Option <Message>
	{
		Some (Message::Text (i))
	}
}

// Maybe make this use exit status instead?  Alternately, Option?
impl OutputFormat for Text
{
	type External = Utf8Bytes;

	fn convert_text (utf8_bytes: Utf8Bytes) -> Option <Self::External>
	{
		Some (utf8_bytes)
	}

	fn convert_binary (bytes: Bytes) -> Option <Self::External>
	{
		event!
		(
			Level::ERROR,
			message_bytes = ?bytes,
			"received binary message in text-only protocol"
		);

		None
	}
}

#[derive (Copy, Clone, Debug)]
pub struct Binary;

impl InputFormat for Binary
{
	type Intermediate = Bytes;

	fn convert (i: Self::Intermediate) -> Option <Message>
	{
		Some (Message::Binary (i))
	}
}

impl OutputFormat for Binary
{
	type External = Bytes;

	fn convert_text (utf8_bytes: Utf8Bytes) -> Option <Self::External>
	{
		event!
		(
			Level::ERROR,
			message_bytes = ?utf8_bytes,
			"received text message in binary-only protocol"
		);

		None
	}

	fn convert_binary (bytes: Bytes) -> Option <Self::External>
	{
		Some (bytes)
	}
}
