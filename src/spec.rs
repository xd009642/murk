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
use std::path::PathBuf;

#[doc(hidden)]
fn one() -> usize {
    1
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Specification {
    pub paths: IndexMap<String, PathItem>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PathItem {
    pub get: Option<Operation>,
    pub post: Option<Operation>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Operation {
    #[serde(default)]
    pub request_data: IndexMap<String, Data>,
    pub request_body: RequestBody,
    #[serde(default)]
    pub parameters: Vec<Parameter>,
    #[serde(default = "one")]
    pub weight: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Data {
    #[serde(default)]
    pub parameters: Vec<TestParameter>,
    pub body: Option<TestBody>,
    #[serde(default = "one")]
    pub weight: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TestParameter {
    Header { name: String, value: String },
    Path(String),
    Query { name: String, value: String },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TestBody {
    Constant(String),
    External(PathBuf),
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_yaml::from_str;

    #[test]
    fn deserialise_specification() {
        let sample_spec = r#"
            paths:
              upload:
                post:
                  requestData:
                    static_string:
                      parameters:
                        - header:
                            name: X-Request-ID
                            value: 77e1c83b-7bb0-437b-bc50-a7a58e5660ac
                      body:
                        external: "I am a files contents"
                    file_upload:
                      body:
                        external: "/home/xd009642/corpus"
                  requestBody:
                    parameters:
                      - in: header
                        name: X-Request-ID
                        schema:
                          type: string
                          format: uuid
                        required: true
                    content:
                      multipart/form-data:
                        schema: 
                          type: object
                          properties:
                            file:
                              type: string
                              format: binary
                      application/octet-stream:
                        schema:
                          type: binary
        "#;

        let _spec: Specification = from_str(sample_spec).unwrap();
    }
}
