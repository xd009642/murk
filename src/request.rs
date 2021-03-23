use crate::spec::{Operation, Specification};
use bytes::Bytes;
use hyper::{Request, Uri};
use random_choice::random_choice;
use url::Url;

pub struct RequestStore {
    /// List of weights. This list will be either be empty or the same length as the requests vector
    pub(crate) weights: Vec<f64>,
    /// List of the requests to use. I'm kind of assuming since I'm using a Bytes to store the body
    /// that these will be relatively cheap to clone... But there's only one way to find out
    pub(crate) requests: Vec<Request<Bytes>>,
}

fn requests_from_operation(uri: Url, op: &Operation) -> (Vec<f64>, Vec<Request<Bytes>>) {
    todo!()
}

impl RequestStore {
    pub fn create_from_spec(url: String, spec: &Specification) -> Self {
        let mut weights = vec![];
        let mut requests = vec![];

        let base_uri = Url::parse(&url).expect("URL invalid");
        for (name, item) in &spec.paths {
            let uri = base_uri.join(&name).expect("Invalid method name");

            if let Some(get) = item.get.as_ref() {
                let (mut w, mut r) = requests_from_operation(uri.clone(), get);
                weights.append(&mut w);
                requests.append(&mut r);
            }
            if let Some(post) = item.post.as_ref() {
                let (mut w, mut r) = requests_from_operation(uri.clone(), post);
                weights.append(&mut w);
                requests.append(&mut r);
            }
        }

        Self { weights, requests }
    }

    pub fn get_request(&self) -> &Request<Bytes> {
        self.get_requests(1).remove(0)
    }

    pub fn get_requests(&self, samples: usize) -> Vec<&Request<Bytes>> {
        assert_ne!(samples, 0, "Samples must be >0");
        assert!(!self.requests.is_empty(), "No request data");
        assert!(
            self.weights.len() == self.requests.len(),
            "Weights vector must match the requests vector"
        );

        random_choice().random_choice_f64(&self.requests, &self.weights, samples)
    }
}
