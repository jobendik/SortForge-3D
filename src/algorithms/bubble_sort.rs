//! Bubble sort: repeatedly walk the array swapping out-of-order neighbors.
//!
//! Event story:
//! * Each adjacent pair check emits a [`Compare`](crate::simulation::SortEvent::Compare).
//! * Out-of-order pairs emit a [`Swap`](crate::simulation::SortEvent::Swap).
//! * After pass `p`, the largest remaining element has "bubbled" to the end of
//!   the unsorted region, so we emit
//!   [`MarkSorted`](crate::simulation::SortEvent::MarkSorted) for it - the green
//!   sorted tail grows by one each pass.
//! * If a pass performs no swaps the array is already ordered, so we mark the
//!   whole remaining prefix sorted and stop early.

use super::{Recorder, SortingAlgorithm};
use crate::simulation::SortEvent;

pub struct BubbleSort;

impl SortingAlgorithm for BubbleSort {
    fn name(&self) -> &'static str {
        "Bubble Sort"
    }

    fn generate_events(&self, values: &[u32]) -> Vec<SortEvent> {
        let mut a = values.to_vec();
        let n = a.len();
        let mut rec = Recorder::with_capacity_for(n);

        if n <= 1 {
            if n == 1 {
                rec.mark_sorted(0);
            }
            return rec.finish();
        }

        let mut finished_early = false;
        for pass in 0..n - 1 {
            // Everything from `last` onward is already sorted from prior passes.
            let last = n - 1 - pass;
            let mut swapped = false;

            for i in 0..last {
                rec.compare(i, i + 1);
                if a[i] > a[i + 1] {
                    rec.swap(&mut a, i, i + 1);
                    swapped = true;
                }
            }

            // The largest unsorted element is now parked at `last`.
            rec.mark_sorted(last);

            if !swapped {
                // No swaps this pass => the unsorted prefix is already ordered.
                for k in 0..last {
                    rec.mark_sorted(k);
                }
                finished_early = true;
                break;
            }
        }

        // After a full set of passes, index 0 is the only element never marked.
        if !finished_early {
            rec.mark_sorted(0);
        }

        rec.finish()
    }
}
