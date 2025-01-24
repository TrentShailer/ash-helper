use parking_lot::{Mutex, MutexGuard};

pub enum MaybeMutex<'m, T: Copy> {
    Raw(T),
    Mutex(&'m Mutex<T>),
}

impl<T: Copy> From<T> for MaybeMutex<'_, T> {
    fn from(value: T) -> Self {
        Self::Raw(value)
    }
}

impl<'m, T: Copy> From<&'m Mutex<T>> for MaybeMutex<'m, T> {
    fn from(value: &'m Mutex<T>) -> Self {
        Self::Mutex(value)
    }
}

impl<'m, T: Copy> MaybeMutex<'m, T> {
    pub fn lock(&self) -> (T, Option<MutexGuard<'m, T>>) {
        match self {
            MaybeMutex::Raw(value) => (*value, None),
            MaybeMutex::Mutex(mutex) => {
                let guard = mutex.lock();
                (*guard, Some(guard))
            }
        }
    }
}
