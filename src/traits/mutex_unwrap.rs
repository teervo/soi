use std::sync::Mutex;
use std::sync::MutexGuard;

/// This trait provides a `lockk()` method for Mutex to be used
/// as a shorthand for the boilerplate `lock().unwrap()`.
pub trait UnwrappedMutex<T> {
    fn lockk(&self) -> MutexGuard<'_, T>;
}

impl<T> UnwrappedMutex<T> for Mutex<T> {
    fn lockk(&self) -> MutexGuard<'_, T> {
        self.lock().unwrap()
    }
}
