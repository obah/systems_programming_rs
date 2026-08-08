use std::fmt;

use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct Point {
    x: i32,
    y: i32,
}

#[derive(Debug, PartialEq)]
enum Message {
    Request {
        id: String,
        method: String,
        length: u16,
    },
    Response {
        id: String,
        result: f64,
    },
}

//output should look like - {"type": "Request", "id": "...", "method": "...", "length": {...}}
fn serialize_enum(value: Message) -> String {
    match value {
        Message::Response { id, result } => {
            format!(r#"{{"type": "Response", "id": "{id}", "result": {result}}}"#)
        }
        Message::Request { id, method, length } => {
            format!(
                r#"{{"type": "Request", "id": "{id}", "method": "{method}", "length": {length}}}"#
            )
        }
    }
}

impl<'de> Deserialize<'de> for Message {
    // Almost no logic lives here. `deserialize`'s only job is to tell the format what
    // shape we expect - a map, since our wire form is {"type": ..., <fields>} - and hand
    // over the visitor. The hint is a *request*: JSON is self-describing so it will look
    // at the actual bytes and may call a different visit_* method; bincode/postcard have
    // no type info on the wire and must obey it.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(MessageVisitor)
    }
}

// Zero-sized: the visitor holds no state, it is just a place to hang the visit_* methods
// serde calls back into. Every method we don't implement defaults to a type error, so this
// type accepts maps and nothing else.
struct MessageVisitor;

impl<'de> Visitor<'de> for MessageVisitor {
    type Value = Message;

    // Serde writes this into the error when the input isn't a shape we accept:
    // "invalid type: integer `3`, expected <this string>".
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str(r#"a map with a "type" of "Request" or "Response""#)
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        // The tag is *internal* - "type" is a sibling of the payload fields, not a wrapper -
        // and JSON maps are unordered, so "type" may arrive last. We can't branch on the
        // variant while reading; buffer every field and decide once the map is drained.
        let mut tag: Option<String> = None;
        let mut id: Option<String> = None;
        let mut method: Option<String> = None;
        let mut length: Option<u16> = None;
        let mut result: Option<f64> = None;

        // MapAccess is a pull-based iterator: next_key() until it returns None, and each key
        // must be followed by its next_value() before the next key.
        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                // next_value() infers what to parse into from the Option it lands in,
                // so "length" is parsed as a real u16, not a string.
                "type" => tag = Some(map.next_value()?),
                "id" => id = Some(map.next_value()?),
                "method" => method = Some(map.next_value()?),
                "length" => length = Some(map.next_value()?),
                "result" => result = Some(map.next_value()?),
                // Unknown keys still need their value consumed, otherwise the next
                // next_key() call would read that dangling value *as* a key.
                // IgnoredAny parses and discards without allocating.
                _ => {
                    map.next_value::<de::IgnoredAny>()?;
                }
            }
        }

        // Which fields are required depends on the variant, so validation happens here rather
        // than per-field. This is the type boundary: the wire cannot construct a Message that
        // the enum's own shape forbids (a Request with no method, an unknown variant).
        match tag.as_deref() {
            Some("Request") => Ok(Message::Request {
                id: id.ok_or_else(|| de::Error::missing_field("id"))?,
                method: method.ok_or_else(|| de::Error::missing_field("method"))?,
                length: length.ok_or_else(|| de::Error::missing_field("length"))?,
            }),
            Some("Response") => Ok(Message::Response {
                id: id.ok_or_else(|| de::Error::missing_field("id"))?,
                result: result.ok_or_else(|| de::Error::missing_field("result"))?,
            }),
            Some(other) => Err(de::Error::unknown_variant(other, &["Request", "Response"])),
            None => Err(de::Error::missing_field("type")),
        }
    }
}

fn deserialize_enum(string: String) -> Message {
    // serde_json drives the Deserialize impl above; all the work is in the visitor.
    serde_json::from_str(&string).expect("not a valid Message")
}

fn main() {
    let point = Point { x: 1, y: 2 };

    // Convert the Point to a JSON string.
    let serialized = serde_json::to_string(&point).unwrap();

    // Prints serialized = {"x":1,"y":2}
    println!("serialized = {}", serialized);

    // Convert the JSON string back to a Point.
    let deserialized: Point = serde_json::from_str(&serialized).unwrap();

    // Prints deserialized = Point { x: 1, y: 2 }
    println!("deserialized = {:?}", deserialized);

    let json = serialize_enum(Message::Request {
        id: "1".into(),
        method: "ping".into(),
        length: 4,
    });
    println!("message = {json}");
    println!("message = {:?}", deserialize_enum(json));
}

//derive Serialize/Deserialize for a config struct, then hand-implement Serialize for one enum so you feel the visitor/data-model boundary

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> Message {
        Message::Request {
            id: "1".into(),
            method: "ping".into(),
            length: 4,
        }
    }

    #[test]
    fn round_trips_both_variants() {
        assert_eq!(deserialize_enum(serialize_enum(request())), request());

        let response = || Message::Response {
            id: "1".into(),
            result: 1.5,
        };
        assert_eq!(deserialize_enum(serialize_enum(response())), response());
    }

    #[test]
    fn tag_order_and_unknown_keys_dont_matter() {
        let json =
            r#"{"extra": [1,2], "length": 4, "id": "1", "type": "Request", "method": "ping"}"#;
        assert_eq!(deserialize_enum(json.to_string()), request());
    }

    #[test]
    fn rejects_bad_input() {
        for bad in [
            r#"{"type": "Request", "id": "1"}"#, // missing method/length
            r#"{"type": "Notify", "id": "1"}"#,  // unknown variant
            r#"{"id": "1"}"#,                    // no tag
            r#"[1, 2]"#,                         // not a map
        ] {
            assert!(serde_json::from_str::<Message>(bad).is_err(), "{bad}");
        }
    }
}
