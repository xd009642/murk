use crate::spec::*;
use bytes::Bytes;
use hyper::{
    header::{HeaderName, HeaderValue},
    Body, HeaderMap, Method, Request,
};
use random_choice::random_choice;
pub use std::convert::TryFrom;
use std::fs;
use std::path::Path;
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

impl TryFrom<String> for RequestBuilder {
    type Error = url::ParseError;

    fn try_from(url: String) -> Result<Self, Self::Error> {
        let url = url.parse::<Url>()?;
        Ok(Self::from(url))
    }
}

impl From<Url> for RequestBuilder {
    fn from(url: Url) -> Self {
        Self {
            url,
            method: Method::GET,
            headers: Default::default(),
            body: Bytes::new(),
        }
    }
}

impl RequestBuilder {
    pub fn request(&self) -> Request<Body> {
        let mut builder = Request::builder()
            .uri(self.url.as_str())
            .method(self.method.clone());
        if let Some(headers) = builder.headers_mut() {
            for (k, v) in &self.headers {
                headers.insert(k, v.clone());
            }
        }
        builder.body(self.body.clone().into()).unwrap()
    }

    pub fn body_len(&self) -> usize {
        self.body.len()
    }
}

fn bodies_from_path(path: &Path) -> Vec<Bytes> {
    if path.is_file() {
        // just open and read all
        vec![Bytes::from(fs::read(path).unwrap())]
    } else if path.is_dir() {
        let mut res = vec![];
        let dir_stream = fs::read_dir(path).unwrap();
        for entry in dir_stream {
            let path = entry.unwrap().path();
            if path.is_file() {
                if let Ok(b) = fs::read(&path) {
                    res.push(Bytes::from(b));
                } else {
                    println!("Couldn't read: {}", path.display());
                }
            }
        }
        res
    } else {
        panic!("Invalid path {} no data found", path.display());
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
                    let mut seg = url.path_segments_mut().expect("URL Cannot be base");
                    seg.push(s);
                }
                TestParameter::Query { name, value } => {
                    url.query_pairs_mut().append_pair(name, value);
                }
            }
        }

        let mut reqs = if let Some(b) = &v.body {
            match b {
                TestBody::Constant(s) => {
                    vec![RequestBuilder {
                        url: url.clone(),
                        method: method.clone(),
                        headers,
                        body: Bytes::from(s.clone()),
                    }]
                }
                TestBody::External(p) => bodies_from_path(&p)
                    .iter()
                    .map(|body| RequestBuilder {
                        url: url.clone(),
                        method: method.clone(),
                        headers: headers.clone(),
                        body: body.clone(),
                    })
                    .collect(),
            }
        } else {
            vec![RequestBuilder {
                url: url.clone(),
                method: method.clone(),
                headers,
                body: Bytes::new(),
            }]
        };
        for _ in 0..reqs.len() {
            weights.push((op.weight * v.weight) as f64);
        }
        requests.append(&mut reqs);
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

    pub fn len(&self) -> usize {
        self.requests.len()
    }
}
