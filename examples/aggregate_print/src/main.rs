//! A sample application continuously aggregating metrics,
//! printing the summary stats every three seconds and
//! printing complete stats every 10 seconds.

extern crate dipstick;

use std::time::Duration;
use dipstick::*;

fn main() {
    let (to_quick_aggregate, from_quick_aggregate) = aggregate();
    let (to_slow_aggregate, from_slow_aggregate) = aggregate();

    let app_metrics = metrics((to_quick_aggregate, to_slow_aggregate));

    publish_every(Duration::from_secs(3), from_quick_aggregate, to_stdout(), publish::summary);

    publish_every(Duration::from_secs(10), from_slow_aggregate, to_stdout(), publish::all_stats);

    let counter = app_metrics.counter("counter_a");
    loop {
        // add counts forever, non-stop
        counter.count(11);
        counter.count(12);
        counter.count(13);
    }

}