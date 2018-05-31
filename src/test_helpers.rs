/* Accepts a pointer to an existing variable.
       Will increment/decrement it when an instance is
       cloned/dropped respectively.
       Used to test for clones and drops. */
pub struct RefCountTestDouble {
    pub ref_count: *mut isize
}

impl Drop for RefCountTestDouble {
    fn drop(&mut self) {
        unsafe {(*self.ref_count) -= 1;}
    }
}

impl Clone for RefCountTestDouble {
    fn clone(&self) -> Self {
        unsafe {(*self.ref_count) += 1;}
        Self {
            ref_count: self.ref_count
        }
    }
}