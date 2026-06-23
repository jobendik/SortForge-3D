//! Insertion sort: grow a sorted prefix by inserting each element into place.
//!
//! This is the *shift* formulation (not the swap formulation), which is what
//! makes it a good showcase for the
//! [`Overwrite`](crate::simulation::SortEvent::Overwrite) event:
//!
//! * We save `key = a[i]` and walk left, emitting a
//!   [`Compare`](crate::simulation::SortEvent::Compare) of the hole against its
//!   left neighbor.
//! * Each neighbor larger than `key` is shifted one slot right via an
//!   `Overwrite` (orange "write" highlight travels right).
//! * Finally `key` is written into the hole with one more `Overwrite`.
//!
//! Note on `MarkSorted`: the prefix is only sorted *relative to itself* while
//! the algorithm runs - later insertions still shift it - so finalizing
//! elements early would be a lie. We therefore mark everything sorted only once
//! the whole array is ordered.

use super::{Recorder, SortingAlgorithm};
use crate::simulation::SortEvent;

pub struct InsertionSort;

impl SortingAlgorithm for InsertionSort {
    fn name(&self) -> &'static str {
        "Insertion Sort"
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

        for i in 1..n {
            let key = a[i];
            let mut j = i;

            // Walk left while the neighbor is bigger than the key, shifting it
            // up into the hole. The comparison uses the saved `key`, while the
            // event highlights the two adjacent bars the user can see.
            while j > 0 {
                rec.compare(j - 1, j);
                if a[j - 1] > key {
                    let shifted = a[j - 1];
                    rec.overwrite(&mut a, j, shifted);
                    j -= 1;
                } else {
                    break;
                }
            }

            // Only write the key if it actually moved (avoids a no-op write).
            if j != i {
                rec.overwrite(&mut a, j, key);
            }
            rec.clear_highlights();
        }

        // The whole array is ordered now: finalize every element.
        for k in 0..n {
            rec.mark_sorted(k);
        }

        rec.finish()
    }
}
