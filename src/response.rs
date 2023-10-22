use std::borrow::Cow;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::Id;

/// A JSON-RPC 2.0 response.
#[derive(Debug, Clone)]
pub struct Response<'a, T, E> {
    /// The outcome of the request.
    pub result: Result<T, Error<'a, E>>,
    /// The ID of the request to which this repsonse is a reply.
    pub id: Id<'a>,
}

impl<'a, T, E> Serialize for Response<'a, T, E>
where
    T: Serialize,
    E: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        OutogingResponse::from_response(self).serialize(serializer)
    }
}

impl<'de, 'a, T, E> Deserialize<'de> for Response<'a, T, E>
where
    'de: 'a,
    T: Deserialize<'de>,
    E: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        IncomingResponse::deserialize(deserializer).and_then(IncomingResponse::into_response)
    }
}

/// A JSON-RPC 2.0 error code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ErrorCode(pub i64);

/// A JSON-RPC 2.0 error.
#[derive(Debug, Clone)]
pub struct Error<'a, E> {
    /// The error code.
    pub code: ErrorCode,
    /// The error message.
    pub message: Cow<'a, str>,
    /// Additional data about the error.
    pub data: Option<E>,
}

#[derive(Serialize)]
struct OutogingResponse<'a, T, E> {
    jsonrpc: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<&'a T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<OutgoingError<'a, E>>,
    id: crate::Id<'a>,
}

impl<'a, T, E> OutogingResponse<'a, T, E> {
    fn from_response(response: &'a crate::Response<T, E>) -> Self {
        let (result, error) = match response.result {
            Ok(ref result) => (Some(result), None),
            Err(ref error) => (
                None,
                Some(OutgoingError {
                    code: error.code.0,
                    message: &error.message,
                    data: error.data.as_ref(),
                }),
            ),
        };

        Self {
            jsonrpc: "2.0",
            result,
            error,
            id: id_as_ref(&response.id),
        }
    }
}

#[derive(Serialize)]
struct OutgoingError<'a, E> {
    code: i64,
    message: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<&'a E>,
}

#[derive(Deserialize)]
struct IncomingResponse<'a, T, E> {
    #[serde(borrow)]
    jsonrpc: Cow<'a, str>,
    #[serde(default = "Option::default")]
    result: Option<T>,
    #[serde(default = "Option::default", borrow)]
    error: Option<IncomingError<'a, E>>,
    // The option is there to represent the `null` value. The `id` field must still be
    // present.
    #[serde(borrow)]
    id: crate::Id<'a>,
}

impl<'a, T, E> IncomingResponse<'a, T, E> {
    fn into_response<Er>(self) -> Result<crate::Response<'a, T, E>, Er>
    where
        Er: serde::de::Error,
    {
        if self.jsonrpc != "2.0" {
            return Err(Er::invalid_value(
                serde::de::Unexpected::Str(&self.jsonrpc),
                &"2.0",
            ));
        }

        let result = match (self.result, self.error) {
            (Some(result), None) => Ok(result),
            (None, Some(error)) => Err(crate::Error {
                code: crate::ErrorCode(error.code),
                message: error.message,
                data: error.data,
            }),
            (Some(_), Some(_)) => {
                return Err(Er::custom(
                    "response cannot contain both `result` and `error` fields",
                ))
            }
            (None, None) => {
                return Err(Er::custom(
                    "response must contain either `result` or `error` field",
                ))
            }
        };

        Ok(crate::Response {
            result,
            id: self.id,
        })
    }
}

#[derive(Deserialize)]
struct IncomingError<'a, E> {
    code: i64,
    #[serde(borrow)]
    message: Cow<'a, str>,
    #[serde(default = "Option::default")]
    data: Option<E>,
}

fn id_as_ref<'a>(id: &'a crate::Id) -> crate::Id<'a> {
    match *id {
        crate::Id::Null => crate::Id::Null,
        crate::Id::Float(f) => crate::Id::Float(f),
        crate::Id::Str(ref s) => crate::Id::Str(Cow::Borrowed(s)),
        crate::Id::Int(i) => crate::Id::Int(i),
        crate::Id::Uint(u) => crate::Id::Uint(u),
    }
}
