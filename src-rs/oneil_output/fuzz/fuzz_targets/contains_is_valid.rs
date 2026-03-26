#![no_main]

use libfuzzer_sys::{arbitrary, fuzz_target};
use shared::NonNanInterval;

#[derive(Debug, Clone, PartialEq, arbitrary::Arbitrary)]
pub struct FuzzData {
    interval1: NonNanInterval,
    interval2: NonNanInterval,
}

fuzz_target!(|data: FuzzData| {
    let NonNanInterval(interval1) = data.interval1;
    let NonNanInterval(interval2) = data.interval2;

    let interval1_contains_interval2 = interval1.contains(&interval2);
    let interval_intersection = interval1.intersection(interval2);

    // check that the intersection is equal to the second interval
    //
    // don't just do `interval_intersection == interval2` because
    // interval comparison uses `is_close`, but we want exact equality
    let interval_intersection_is_interval2 = interval_intersection.min() == interval2.min()
        && interval_intersection.max() == interval2.max();

    if interval1_contains_interval2 && !interval_intersection_is_interval2 {
        panic!(
            "interval1 ({:?}) contains interval2 ({:?}) but interval intersection ({:?}) is not equal to interval2 ({:?})",
            interval1, interval2, interval_intersection, interval2,
        );
    }

    if !interval1_contains_interval2 && interval_intersection_is_interval2 {
        panic!(
            "interval1 ({:?}) does not contain interval2 ({:?}) but interval intersection ({:?}) is equal to interval2 ({:?})",
            interval1, interval2, interval_intersection, interval2,
        );
    }
});
