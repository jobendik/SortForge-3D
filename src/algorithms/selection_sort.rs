//! Selection sort: repeatedly find the minimum of the unsorted region and
//! place it at the front.
//!
//! Event story:
//! * The running minimum is highlighted with
//!   [`SetPivot`](crate::simulation::SortEvent::SetPivot) (magenta) and moves as
//!   a smaller candidate is found.
//! * Each scan step emits a [`Compare`](crate::simulation::SortEvent::Compare)
//!   of the candidate against the current minimum.
//! * Once the true minimum is known it is swapped into position `i` with a
//!   [`Swap`](crate::simulation::SortEvent::Swap) (skipped when already there).
//! * Index `i` then holds the i-th smallest value for good, so it is marked
//!   sorted - the green region grows from the left.

use super::{Recorder, SortingAlgorithm};
use crate::simulation::SortEvent;

pub struct SelectionSort;

impl SortingAlgorithm for SelectionSort {
    fn name(&self) -> &'static str {
        "Selection Sort"
    }

    fn generate_events(&self, values: &[u32]) -> Vec<SortEvent> {
        let mut a = values.to_vec();
        let n = a.len();
        let mut rec = Recorder::with_capacity_for(n);

        for i in 0..n {
            let mut min = i;
            rec.set_pivot(min);

            for j in (i + 1)..n {
                rec.compare(j, min);
                if a[j] < a[min] {
                    min = j;
                    // Move the magenta "current minimum" highlight.
                    rec.set_pivot(min);
                }
            }

            if min != i {
                rec.swap(&mut a, i, min);
            }

            // Drop the pivot highlight, then lock index `i` in as sorted.
            rec.clear_highlights();
            rec.mark_sorted(i);
        }

        rec.finish()
    }
}
