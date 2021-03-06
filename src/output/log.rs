use core::{Flush};
use core::input::{InputKind, Input, InputScope, InputMetric};
use core::attributes::{Attributes, WithAttributes, Buffered, Prefixed};
use core::name::MetricName;
use core::error;
use cache::cache_in;
use queue::queue_in;
use output::format::{LineFormat, SimpleFormat, Formatting};

use std::sync::{RwLock, Arc};
use std::io::Write;
use log;

/// Buffered metrics log output.
#[derive(Clone)]
pub struct Log {
    attributes: Attributes,
    format: Arc<LineFormat>,
}

impl Input for Log {
    type SCOPE = LogScope;

    fn input(&self) -> Self::SCOPE {
        LogScope {
            attributes: self.attributes.clone(),
            entries: Arc::new(RwLock::new(Vec::new())),
            output: self.clone(),
        }
    }
}

impl WithAttributes for Log {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}

impl Buffered for Log {}

impl Formatting for Log {
    fn formatting(&self, format: impl LineFormat + 'static) -> Self {
        let mut cloned = self.clone();
        cloned.format = Arc::new(format);
        cloned
    }
}

/// A scope for metrics log output.
#[derive(Clone)]
pub struct LogScope {
    attributes: Attributes,
    entries: Arc<RwLock<Vec<Vec<u8>>>>,
    output: Log,
}

impl Log {
    /// Write metric values to the standard log using `info!`.
    // TODO parameterize log level, logger
    pub fn log_to() -> Log {
        Log {
            attributes: Attributes::default(),
            format: Arc::new(SimpleFormat::default()),
        }
    }
}

impl WithAttributes for LogScope {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}

impl Buffered for LogScope {}

impl queue_in::QueuedInput for Log {}
impl cache_in::CachedInput for Log {}

impl InputScope for LogScope {
    fn new_metric(&self, name: MetricName, kind: InputKind) -> InputMetric {
        let name = self.prefix_append(name);

            let template = self.output.format.template(&name, kind);

        let entries = self.entries.clone();

        if let Some(_buffering) = self.get_buffering() {
            InputMetric::new(move |value, labels| {
                let mut buffer = Vec::with_capacity(32);
                match template.print(&mut buffer, value, |key| labels.lookup(key)) {
                    Ok(()) => {
                        let mut entries = entries.write().expect("TextOutput");
                        entries.push(buffer)
                    },
                    Err(err) => debug!("Could not format buffered log metric: {}", err),
                }
            })
        } else {
            // unbuffered
            InputMetric::new(move |value, labels| {
                let mut buffer = Vec::with_capacity(32);
                match template.print(&mut buffer, value, |key| labels.lookup(key)) {
                    Ok(()) => log!(log::Level::Debug, "{:?}", &buffer),
                    Err(err) => debug!("Could not format buffered log metric: {}", err),
                }
            })
        }
    }
}

impl Flush for LogScope {

    fn flush(&self) -> error::Result<()> {
        let mut entries = self.entries.write().expect("Metrics TextBuffer");
        if !entries.is_empty() {
            let mut buf: Vec<u8> = Vec::with_capacity(32 * entries.len());
            for entry in entries.drain(..) {
                writeln!(&mut buf, "{:?}", &entry)?;
            }
            log!(log::Level::Debug, "{:?}", &buf);
        }
        Ok(())
    }
}

impl Drop for LogScope {
    fn drop(&mut self) {
        if let Err(e) = self.flush() {
            warn!("Could not flush log metrics on Drop. {}", e)
        }
    }
}

#[cfg(test)]
mod test {
    use core::input::*;

    #[test]
    fn test_to_log() {
        let c = super::Log::log_to().input();
        let m = c.new_metric("test".into(), InputKind::Marker);
        m.write(33, labels![]);
    }

}
