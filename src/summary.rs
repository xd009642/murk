use hdrhistogram::Histogram;
use hyper::StatusCode;
use std::collections::BTreeMap;
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
    pub status_codes: BTreeMap<u16, usize>,
    pub histogram: Histogram<u64>,
    pub custom_histograms: BTreeMap<String, Histogram<u64>>,
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
            custom_histograms: BTreeMap::new(),
            status_codes: BTreeMap::new(),
        }
    }

    pub fn register_custom_histogram(&mut self, name: String, min: u64, max: u64, accuracy: u8) {
        let hist = Histogram::<u64>::new_with_bounds(min, max, accuracy).unwrap();
        self.custom_histograms.insert(name, hist);
    }
}

impl fmt::Display for Summary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Successful requests: {}", self.success)?;
        writeln!(f, "Failed requests: {}", self.failure)?;
        writeln!(f, "Timed out requests: {}", self.timeout)?;
        writeln!(f, "Bytes read: {}", self.bytes_read)?;
        writeln!(f, "Bytes written: {}", self.bytes_written)?;
        writeln!(f, "\nQuantile durations:")?;
        let quantiles = [0.5, 0.75, 0.9, 0.95, 0.99, 0.999];
        for quant in &quantiles {
            writeln!(
                f,
                "{}'th percentile: {}",
                *quant * 100.0,
                self.histogram.value_at_quantile(*quant)
            )?;
        }
        Ok(())
    }
}

impl std::ops::AddAssign for Summary {
    fn add_assign(&mut self, other: Self) {
        self.success += other.success;
        self.failure += other.failure;
        self.timeout += other.timeout;
        self.bytes_read += other.bytes_read;
        self.bytes_written += other.bytes_written;
        self.histogram.add(other.histogram).unwrap();
        for (k, v) in self.status_codes.iter_mut() {
            if let Some(v2) = other.status_codes.get(k) {
                *v += v2;
            }
        }
        for (k, v) in self.custom_histograms.iter_mut() {
            if let Some(v2) = other.custom_histograms.get(k) {
                v.add(v2).unwrap();
            }
        }
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

    fn add(mut self, other: Self) -> Self {
        let mut histogram = self.histogram.clone();
        histogram.add(other.histogram).unwrap();
        for (k, v) in self.status_codes.iter_mut() {
            if let Some(v2) = other.status_codes.get(k) {
                *v += v2;
            }
        }
        for (k, v) in self.custom_histograms.iter_mut() {
            if let Some(v2) = other.custom_histograms.get(k) {
                v.add(v2).unwrap();
            }
        }
        Self {
            histogram,
            success: self.success + other.success,
            failure: self.failure + other.failure,
            timeout: self.timeout + other.timeout,
            bytes_read: self.bytes_read + other.bytes_read,
            bytes_written: self.bytes_written + other.bytes_written,
            status_codes: self.status_codes,
            custom_histograms: self.custom_histograms,
        }
    }
}
