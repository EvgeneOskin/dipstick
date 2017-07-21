use core::{MetricType, RateType, Value, MetricSink, SinkMetric, SinkWriter};
use cached::{SizedCache, Cached};
use std::sync::Arc;
use std::sync::RwLock;

// METRIC

// TODO get rid of this struct+impl, replace it with
// impl <C: MetricSink> SinkMetric for Arc<C::Metric> {}
// and let sink just return a clone the cached Arc
// which I could not make it compile because:
// "the type parameter `C` is not constrained by the impl trait, self type, or predicates E0207 unconstrained type parameter"
// which is strange because Arc<C::Metric> looks like a <C> constraint to me...
// one solution might require SinkMetric<PHANTOM> (everywhere!), not tried because it would be HORRIBLE
// for now we use this "wrapping reification" of Arc<> which needs to be allocated on every cache miss or hit
// if you know how to fix it that'd be great
pub struct CachedMetric<C: MetricSink> (Arc<C::Metric>);

impl <C: MetricSink> SinkMetric for CachedMetric<C> {}

// WRITER

pub struct CachedMetricWriter<C: MetricSink> {
    target: C,
}

impl <C: MetricSink> SinkWriter<CachedMetric<C>> for CachedMetricWriter<C> {
    fn write(&self, metric: &CachedMetric<C>, value: Value) {
        self.target.write(|scope| scope.write(metric.0.as_ref(), value))
    }
}

// SINK

pub struct MetricCache<C: MetricSink> {
    writer: CachedMetricWriter<C>,
    cache: RwLock<SizedCache<String, Arc<C::Metric>>>,
}

impl <C: MetricSink> MetricCache<C> {
    pub fn new(target: C, cache_size: usize) -> MetricCache<C> {
        let cache = RwLock::new(SizedCache::with_capacity(cache_size));
        MetricCache { writer: CachedMetricWriter{ target }, cache,  }
    }
}

impl <C: MetricSink> MetricSink for MetricCache<C> {
    type Metric = CachedMetric<C>;
    type Write = CachedMetricWriter<C>;

    fn define<S: AsRef<str>>(&self, m_type: MetricType, name: S, sampling: RateType) -> CachedMetric<C> {
        let key = name.as_ref().to_string();
        {
            let mut cache = self.cache.write().unwrap();
            let cached_metric = cache.cache_get(&key);
            if let Some(cached_metric) = cached_metric {
                println!("cache hit");
                return CachedMetric(cached_metric.clone());
            }
        }
        println!("cache miss");
        let target_metric = self.writer.target.define(m_type, name, sampling);
        let new_metric = Arc::new( target_metric );
        let mut cache = self.cache.write().unwrap();
        cache.cache_set(key, new_metric.clone());
        CachedMetric(new_metric)
    }

    fn write<F>(&self, operations: F ) where F: Fn(&Self::Write) {
        operations(&self.writer)
    }
}