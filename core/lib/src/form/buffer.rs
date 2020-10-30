use std::cell::UnsafeCell;

use parking_lot::{RawMutex, lock_api::RawMutex as _};

pub(crate) struct Buffer {
    strings: UnsafeCell<Vec<String>>,
    mutex: RawMutex,
}

impl Buffer {
    pub fn new() -> Self {
        Buffer {
            strings: UnsafeCell::new(vec![]),
            mutex: RawMutex::INIT,
        }
    }

    pub fn push_one<'a, S: Into<String>>(&'a self, string: S) -> &'a str {
        // SAFETY:
        //   * Aliasing: We retrieve a mutable reference to the last slot (via
        //     `push()`) and then return said reference as immutable; these
        //     occur in serial, so they don't alias. This method accesses a
        //     unique slot each call: the last slot, subsequently replaced by
        //     `push()` for the next call. No other method accesses the internal
        //     buffer directly. Thus, the outstanding reference to the last slot
        //     is never accessed again mutably, preserving aliasing guarantees.
        //   * Liveness: The returned reference is to a `String`; we must ensure
        //     that the `String` is never dropped while `self` lives. This is
        //     guaranteed by returning a reference with the same lifetime as
        //     `self`, so `self` can't be dropped while the string is live, and
        //     by never removing elements from the internal `Vec` thus not
        //     dropping `String` itself: `push()` is the only mutating operation
        //     called on `Vec`, which preserves all previous elements; the
        //     stability of `String` itself means that the returned address
        //     remains valid even after internal realloc of `Vec`.
        //   * Thread-Safety: Parallel calls without exclusion to `push_one`
        //     would result in a race to `push()`; `RawMutex` ensures that this
        //     doesn't occur.
        unsafe {
            self.mutex.lock();
            let vec: &mut Vec<String> = &mut *self.strings.get();
            vec.push(string.into());
            let last = vec.last().unwrap();
            self.mutex.unlock();
            last
        }
    }

    pub fn push_split(&self, string: String, len: usize) -> (&str, &str) {
        let buffered = self.push_one(string);
        let a = &buffered[..len];
        let b = &buffered[len..];
        (a, b)
    }

    pub fn push_two<'a>(&'a self, a: &str, b: &str) -> (&'a str, &'a str) {
        let mut buffer = String::new();
        buffer.push_str(a);
        buffer.push_str(b);

        self.push_split(buffer, a.len())
    }
}

unsafe impl Sync for Buffer {}
