//! Reduce the amount of data to process or transfer by statistically dropping some of it.

use core::*;
use pcg32;

use std::marker::PhantomData;
use std::sync::Arc;

/// Perform random sampling of values according to the specified rate.
pub fn sample<'ph, M, S>(sampling_rate: Rate, sink: S) -> SamplingSink<'ph, M, S>
    where S: Sink<M>, M: Send + Sync
{
    SamplingSink { next_sink: sink, sampling_rate, phantom: PhantomData {} }
}

/// The metric sampling key also holds the sampling rate to apply to it.
#[derive(Debug)]
pub struct SamplingMetric<M> {
    target: M,
    int_sampling_rate: u32,
}

/// A sampling sink adapter.
pub struct SamplingSink<'ph, M: 'ph, S> {
    next_sink: S,
    sampling_rate: Rate,
    phantom: PhantomData<&'ph M>,
}

impl<'ph, M, S> Sink<SamplingMetric<M>> for SamplingSink<'ph, M, S>
    where S: Sink<M>, M: 'static + Send + Sync
{
    #[allow(unused_variables)]
    fn new_metric(&self, kind: Kind, name: &str, sampling: Rate) -> SamplingMetric<M> {
        // TODO override only if FULL_SAMPLING else warn!()
        assert_eq!(sampling, FULL_SAMPLING_RATE, "Overriding previously set sampling rate");

        let pm = self.next_sink.new_metric(kind, name, self.sampling_rate);
        SamplingMetric {
            target: pm,
            int_sampling_rate: pcg32::to_int_rate(self.sampling_rate),
        }
    }

    fn new_scope(&self) -> ScopeFn<SamplingMetric<M>> {
        let next_scope = self.next_sink.new_scope();
        Arc::new(move |cmd| {
            if let Scope::Write(metric, value) = cmd {
                if pcg32::accept_sample(metric.int_sampling_rate) {
                    next_scope(Scope::Write(&metric.target, value))
                }
            }
            next_scope(Scope::Flush)
        })
    }
}
