use hdrhistogram::Histogram;
use hyper::StatusCode;
use std::fmt;
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq, Ord, PartialOrd)]
pub struct RequestStats {
    pub request_time: Option<Duration>,
    pub status: Option<StatusCode>,
    pub bytes_read: Option<usize>,
    pub bytes_written: Option<usize>,
    pub timeout: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Summary {
    pub success: usize,
    pub failure: usize,
    pub timeout: usize,
    pub bytes_read: usize,
    pub bytes_written: usize,
    pub histogram: Histogram<u64>,
}

impl Summary {
    pub fn new(timeout: Duration) -> Self {
        Self {
            // This should maybe have a buffer
            histogram: Histogram::<u64>::new_with_max(timeout.as_millis() as u64, 3).unwrap(),
            success: 0,
            failure: 0,
            timeout: 0,
            bytes_read: 0,
            bytes_written: 0,
        }
    }
}

impl fmt::Display for Summary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Successful requests: {}", self.success)?;
        writeln!(f, "Failed requests: {}", self.failure)?;
        writeln!(f, "Timed out requests: {}", self.timeout)?;
        writeln!(f, "Bytes read: {}", self.bytes_read)?;
        writeln!(f, "Bytes written: {}", self.bytes_written)
    }
}

impl std::ops::AddAssign for Summary {
    fn add_assign(&mut self, other: Self) {
        self.success += other.success;
        self.failure += other.failure;
        self.timeout += other.timeout;
        self.bytes_read += other.bytes_read;
        self.bytes_written += other.bytes_written;
        self.histogram.add(other.histogram);
    }
}

impl std::ops::AddAssign<RequestStats> for Summary {
    fn add_assign(&mut self, stat: RequestStats) {
        self.bytes_read += stat.bytes_read.unwrap_or_default();
        self.bytes_written += stat.bytes_written.unwrap_or_default();
        if stat.timeout {
            self.timeout += 1;
        } else if let Some(code) = stat.status {
            self.success += code.is_success() as usize;
            self.failure += !code.is_success() as usize;
            if let Some(time) = stat.request_time.map(|x| x.as_millis() as u64) {
                self.histogram.record(time);
            }
        }
    }
}

impl std::ops::Add for Summary {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        let mut histogram = self.histogram.clone();
        histogram.add(other.histogram);
        Self {
            histogram,
            success: self.success + other.success,
            failure: self.failure + other.failure,
            timeout: self.timeout + other.timeout,
            bytes_read: self.bytes_read + other.bytes_read,
            bytes_written: self.bytes_written + other.bytes_written,
        }
    }
}
