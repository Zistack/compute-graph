use std::fmt::Debug;

use serde::{Serialize, Deserialize};
use tracing::{Level, event};

use crate::{service, expand_streams, event_loop, feed};
use crate::exit_status::AlwaysClean;

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
