use bytes::Bytes;
use hyper::Request;
use random_choice::random_choice;

pub struct RequestStore {
    /// List of weights. This list will be either be empty or the same length as the requests vector
    pub(crate) weights: Vec<f64>,
    /// List of the requests to use. I'm kind of assuming since I'm using a Bytes to store the body
    /// that these will be relatively cheap to clone... But there's only one way to find out
    pub(crate) requests: Vec<Request<Bytes>>,
}

impl RequestStore {
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
