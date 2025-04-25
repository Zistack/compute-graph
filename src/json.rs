use std::fmt::Debug;
use std::marker::PhantomData;

use bytes::Bytes;
use serde::{Serialize, Deserialize};
use tracing::{Level, event};
use tungstenite::{Message, Utf8Bytes};

use crate::{service, expand_streams, event_loop, feed};
use crate::exit_status::AlwaysClean;
use crate::websocket::io_format::{InputFormat, OutputFormat};

use crate as compute_graph;

#[expand_streams]
#[service]
pub async fn json_serialize <IS, OS>
(
	item_stream: input! (IS -> impl Serialize + Debug),
	string_sink: output! (OS <- String)
)
-> AlwaysClean
{
	event_loop!
	{
		item_stream -> item => match serde_json::to_string (&item)
		{
			Ok (item_string) => feed! (string_sink, item_string),
			Err (serde_error) => event!
			(
				Level::ERROR,
				item = ?item,
				error = %serde_error,
				"failed to serialize item"
			) . into ()
		}
	}
}

#[expand_streams]
#[service]
pub async fn json_deserialize <IS, OS, OI>
(
	string_stream: input! (IS -> impl AsRef <str>),
	item_sink: output! (OS <- OI: for <'de> Deserialize <'de>)
)
-> AlwaysClean
{
	event_loop!
	{
		string_stream -> string => match serde_json::from_str (string . as_ref ())
		{
			Ok (item) => feed! (item_sink, item),
			Err (serde_error) => event!
			(
				Level::ERROR,
				item_string = ?string . as_ref (),
				error = %serde_error,
				"failed to deserialize item"
			) . into ()
		}
	}
}

struct JSON <T> (PhantomData <T>);

impl <T> InputFormat for JSON <T>
where T: Serialize + Debug
{
	type Intermediate = T;

	fn convert (item: Self::Intermediate) -> Option <Message>
	{
		match serde_json::to_string (&item)
		{
			Ok (item_string) => Some (Message::Text (item_string . into ())),
			Err (serde_error) =>
			{
				event!
				(
					Level::ERROR,
					item_string = ?item,
					error = %serde_error,
					"failed to serialize item"
				);

				None
			}
		}
	}
}

impl <T> OutputFormat for JSON <T>
where T: for <'de> Deserialize <'de>
{
	type External = T;

	fn convert_text (utf8_bytes: Utf8Bytes) -> Option <Self::External>
	{
		let item_string: &str = utf8_bytes . as_ref ();

		match serde_json::from_str (item_string)
		{
			Ok (item) => Some (item),
			Err (serde_error) =>
			{
				event!
				(
					Level::ERROR,
					item_string = ?item_string,
					error = %serde_error,
					"failed to deserialize item"
				);

				None
			}
		}
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
