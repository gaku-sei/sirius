use std::{cmp::Ordering, hint};

// Copied from std, passes an index to the provided predicate function
#[inline]
#[expect(clippy::cast_lossless)]
pub fn binary_search_by_with_index<'a, T, F>(data: &'a [T], mut f: F) -> Result<usize, usize>
where
    F: FnMut(usize, &'a T) -> Ordering,
{
    let mut size = data.len();
    if size == 0 {
        return Err(0);
    }
    let mut base = 0usize;

    // This loop intentionally doesn't have an early exit if the comparison
    // returns Equal. We want the number of loop iterations to depend *only*
    // on the size of the input slice so that the CPU can reliably predict
    // the loop count.
    while size > 1 {
        let half = size / 2;
        let mid = base + half;

        // SAFETY: the call is made safe by the following inconstants:
        // - `mid >= 0`: by definition
        // - `mid < size`: `mid = size / 2 + size / 4 + size / 8 ...`
        let cmp = f(mid, unsafe { data.get_unchecked(mid) });

        // Binary search interacts poorly with branch prediction, so force
        // the compiler to use conditional moves if supported by the target
        // architecture.
        base = select_unpredictable(cmp == Ordering::Greater, base, mid);

        // This is imprecise in the case where `size` is odd and the
        // comparison returns Greater: the mid element still gets included
        // by `size` even though it's known to be larger than the element
        // being searched for.
        //
        // This is fine though: we gain more performance by keeping the
        // loop iteration count invariant (and thus predictable) than we
        // lose from considering one additional element.
        size -= half;
    }

    // SAFETY: base is always in [0, size) because base <= mid.
    let cmp = f(base, unsafe { data.get_unchecked(base) });
    if cmp == Ordering::Equal {
        // SAFETY: same as the `get_unchecked` above.
        unsafe { hint::assert_unchecked(base < data.len()) };
        Ok(base)
    } else {
        let result = base + (cmp == Ordering::Less) as usize;
        // SAFETY: same as the `get_unchecked` above.
        // Note that this is `<=`, unlike the assume in the `Ok` path.
        unsafe { hint::assert_unchecked(result <= data.len()) };
        Err(result)
    }
}

#[inline]
fn select_unpredictable<T>(b: bool, true_val: T, false_val: T) -> T {
    if b {
        true_val
    } else {
        false_val
    }
}
