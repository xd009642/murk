//! This defines a way of specifying jobs for the load test to run through. In order to ease
//! interoperability with existing things this uses an adaptation of OpenAPI v3 to specify the
//! paths to hit in the load test. I strip out the descriptions, tags and other things unimportant
//! for creating the client requests from the requestBody, operation and pathItem objects. Also in
//! the operation object I add a requestData object which is a map with a unique name for each
//! datum that can be sent to the method. This can be a value or an external_value where
//! external_value is a path to a file or folder.
//!
//! I'll also omit things that are in OpenAPI if I don't want to think about how to create the
//! requests or if I have no use for them. They may get added later but who knows.
use indexmap::IndexMap;
use openapiv3::{Parameter, RequestBody};
use serde::{Deserialize, Serialize};
use serde_json::value::Value;
use std::path::PathBuf;

#[doc(hidden)]
fn one() -> usize {
    1
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Specification {
    paths: IndexMap<String, PathItem>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PathItem {
    get: Option<Operation>,
    post: Option<Operation>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Operation {
    request_data: IndexMap<String, Data>,
    request_body: RequestBody,
    parameters: Vec<Parameter>,
    #[serde(default = "one")]
    weight: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Data {
    value: Option<Value>,
    external_value: Option<PathBuf>,
    #[serde(default = "one")]
    weight: usize,
}
