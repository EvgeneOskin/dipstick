//! Send metrics to a statsd server.

use ::core::*;
use ::error;

use std::io::Result;
use std::net::UdpSocket;
use std::cell::RefCell;
use std::sync::Arc;

pub use std::net::ToSocketAddrs;

/// Send metrics to a statsd server at the address and port provided.
pub fn statsd<STR, ADDR>(address: ADDR, prefix: STR) -> error::Result<StatsdSink>
    where STR: AsRef<str>, ADDR: ToSocketAddrs
{
    Ok(StatsdSink::new(address, prefix)?)
}


/// Key of a statsd metric.
#[derive(Debug)]
pub struct StatsdMetric {
    prefix: String,
    suffix: String,
    scale: u64,
}

/// Use a safe maximum size for UDP to prevent fragmentation.
const MAX_UDP_PAYLOAD: usize = 576;

thread_local! {
    static SEND_BUFFER: RefCell<String> = RefCell::new(String::with_capacity(MAX_UDP_PAYLOAD));
}

/// The statsd writer formats metrics to statsd protocol and writes them to a UDP socket.
pub struct StatsdWriter {
    socket: Arc<UdpSocket>,
}

fn flush(payload: &mut String, socket: &UdpSocket) {
    debug!("statsd sending {} bytes", payload.len());
    // TODO check for and report any send() error
    match socket.send(payload.as_bytes()) {
        Ok(size) => { /* TODO inner metrics */ },
        Err(e) => { /* TODO metric faults */ }
    };
    payload.clear();
}

impl Writer<StatsdMetric> for StatsdWriter {
    fn write(&self, metric: &StatsdMetric, value: Value) {
        let scaled_value = if metric.scale != 1 {
            value / metric.scale
        } else {
            value
        };
        let value_str = scaled_value.to_string();
        let entry_len = metric.prefix.len() + value_str.len() + metric.suffix.len();

        SEND_BUFFER.with(|cell| {
            let ref mut buf = cell.borrow_mut();
            if entry_len > buf.capacity() {
                // TODO report entry too big to fit in buffer (!?)
                return;
            }

            let remaining = buf.capacity() - buf.len();
            if entry_len + 1 > remaining {
                // buffer is full, flush before appending
                flush(buf, &self.socket);
            } else {
                if !buf.is_empty() {
                    // separate from previous entry
                    buf.push('\n')
                }
                buf.push_str(&metric.prefix);
                buf.push_str(&value_str);
                buf.push_str(&metric.suffix);
            }
        });
    }

    fn flush(&self) {
        SEND_BUFFER.with(|cell| {
            let ref mut buf = cell.borrow_mut();
            if !buf.is_empty() {
                // operation complete, flush any metrics in buffer
                flush(buf, &self.socket)
            }
        })
    }
}

impl Drop for StatsdWriter {
    fn drop(&mut self) {
        self.flush();
    }
}

/// Allows sending metrics to a statsd server
pub struct StatsdSink {
    socket: Arc<UdpSocket>,
    prefix: String,
}

impl StatsdSink {
    /// Create a new statsd sink to the specified address with the specified prefix
    pub fn new<S: AsRef<str>, A: ToSocketAddrs>(address: A, prefix_str: S) -> Result<StatsdSink> {
        let socket = Arc::new(UdpSocket::bind("0.0.0.0:0")?); // NB: CLOEXEC by default
        socket.set_nonblocking(true)?;
        socket.connect(address)?;
        info!("statsd connected");

        Ok(StatsdSink {
            socket,
            prefix: prefix_str.as_ref().to_string(),
        })
    }
}

impl Sink<StatsdMetric, StatsdWriter> for StatsdSink {

    fn new_metric<S: AsRef<str>>(&self, kind: MetricKind, name: S, sampling: Rate) -> StatsdMetric {
        let mut prefix = String::with_capacity(32);
        prefix.push_str(&self.prefix);
        prefix.push_str(name.as_ref());
        prefix.push(':');

        let mut suffix = String::with_capacity(16);
        suffix.push('|');
        suffix.push_str(match kind {
            MetricKind::Event | MetricKind::Count => "c",
            MetricKind::Gauge => "g",
            MetricKind::Time => "ms",
        });

        if sampling < FULL_SAMPLING_RATE {
            suffix.push('@');
            suffix.push_str(&sampling.to_string());
        }

        let scale = match kind {
            MetricKind::Time => 1000,
            _ => 1
        };

        StatsdMetric { prefix, suffix, scale }
    }

    fn new_writer(&self) -> StatsdWriter {
        StatsdWriter { socket: self.socket.clone() }
    }
}
