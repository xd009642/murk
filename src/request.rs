use crate::spec::*;
use bytes::Bytes;
use hyper::{
    header::{HeaderName, HeaderValue},
    HeaderMap, Method, Request,
};
use random_choice::random_choice;
use url::Url;

pub struct RequestStore {
    /// List of weights. This list will be either be empty or the same length as the requests vector
    pub(crate) weights: Vec<f64>,
    /// List of the requests to use. I'm kind of assuming since I'm using a Bytes to store the body
    /// that these will be relatively cheap to clone... But there's only one way to find out
    pub(crate) requests: Vec<RequestBuilder>,
}

#[derive(Clone)]
pub struct RequestBuilder {
    url: Url,
    method: Method,
    headers: HeaderMap,
    body: Bytes,
}

impl RequestBuilder {
    fn request(&self) -> Request<Bytes> {
        let mut builder = Request::builder()
            .uri(self.url.as_str())
            .method(self.method.clone());
        if let Some(headers) = builder.headers_mut() {
            for (k, v) in &self.headers {
                headers.insert(k, v.clone());
            }
        }
        builder.body(self.body.clone()).unwrap()
    }
}

fn requests_from_operation(
    url: Url,
    method: Method,
    op: &Operation,
) -> (Vec<f64>, Vec<RequestBuilder>) {
    let mut weights = vec![];
    let mut requests = vec![];
    for (k, v) in &op.request_data {
        let mut url = url.clone();
        weights.push((op.weight * v.weight) as f64);
        let mut headers = HeaderMap::new();
        for param in &v.parameters {
            match param {
                TestParameter::Header { name, value } => {
                    let name = HeaderName::from_bytes(name.as_bytes()).unwrap();
                    let value = HeaderValue::from_str(value.as_str()).unwrap();
                    headers.insert(name, value);
                }
                TestParameter::Path(s) => {
                    // Join onto the url
                }
                TestParameter::Query { name, value } => {}
            }
        }

        requests.push(RequestBuilder {
            url: url.clone(),
            method: method.clone(),
            headers: HeaderMap::new(),
            body: Bytes::new(),
        });
    }
    (weights, requests)
}

impl RequestStore {
    pub fn create_from_spec(url: String, spec: &Specification) -> Self {
        let mut weights = vec![];
        let mut requests = vec![];

        let base_uri = Url::parse(&url).expect("URL invalid");
        for (name, item) in &spec.paths {
            let uri = base_uri.join(&name).expect("Invalid method name");

            if let Some(get) = item.get.as_ref() {
                let (mut w, mut r) = requests_from_operation(uri.clone(), Method::GET, get);
                weights.append(&mut w);
                requests.append(&mut r);
            }
            if let Some(post) = item.post.as_ref() {
                let (mut w, mut r) = requests_from_operation(uri.clone(), Method::POST, post);
                weights.append(&mut w);
                requests.append(&mut r);
            }
        }

        Self { weights, requests }
    }

    pub fn get_request(&self) -> &RequestBuilder {
        self.get_requests(1).remove(0)
    }

    pub fn get_requests(&self, samples: usize) -> Vec<&RequestBuilder> {
        assert_ne!(samples, 0, "Samples must be >0");
        assert!(!self.requests.is_empty(), "No request data");
        assert!(
            self.weights.len() == self.requests.len(),
            "Weights vector must match the requests vector"
        );

        random_choice().random_choice_f64(&self.requests, &self.weights, samples)
    }
}
