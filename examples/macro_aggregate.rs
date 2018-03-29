//! A sample application sending ad-hoc counter values both to statsd _and_ to stdout.

extern crate dipstick;
#[macro_use]
extern crate lazy_static;

use dipstick::*;
use std::time::Duration;

// undeclared root (un-prefixed) metrics
aggregate_metrics!(() => {
    // create counter "some_counter"
    pub @Counter ROOT_COUNTER: "root_counter";
    // create counter "root_counter"
    pub @Gauge ROOT_GAUGE: "root_gauge";
    // create counter "root_timer"
    pub @Timer ROOT_TIMER: "root_timer";
});

// public source
aggregate_metrics!(pub PUB_METRICS ="pub_lib_prefix" => {
    // create counter "lib_prefix.some_counter"
    pub @Counter PUB_COUNTER: "some_counter";
});

// undeclared (private) prefixed metrics
//app_metrics!("closed_lib_prefix" => {
//    // create counter "lib_prefix.some_counter"
//    pub @Counter MY_COUNTER: "some_counter";
//});

// declare mod source
aggregate_metrics!(LIB_METRICS ="mod_lib_prefix" => {
    // create counter "mod_lib_prefix.some_counter"
    pub @Counter SOME_COUNTER: "some_counter";
});

// reuse declared source
aggregate_metrics!(LIB_METRICS => {
    // create counter "mod_lib_prefix.another_counter"
    @Counter ANOTHER_COUNTER: "another_counter";
});

fn main() {
    default_aggregate_config(to_stdout());

    loop {
        PUB_COUNTER.count(978);
        ROOT_COUNTER.count(123);
        ANOTHER_COUNTER.count(456);
        ROOT_TIMER.interval_us(2000000);
        ROOT_GAUGE.value(34534);

        PUB_METRICS.flush();
        std::thread::sleep(Duration::from_millis(40));
    }
}
