use std::cell::Cell;
use std::ops::Deref;
use std::sync::OnceLock;

use auditor::domain::ValidName;

pub static KEY_PODNAME: Lazy<ValidName> =
    Lazy::new(|| ValidName::parse("podname".to_owned()).unwrap());
pub static KEY_NAMESPACE: Lazy<ValidName> =
    Lazy::new(|| ValidName::parse("namespace".to_owned()).unwrap());
pub static KEY_STATUS: Lazy<ValidName> =
    Lazy::new(|| ValidName::parse("status".to_owned()).unwrap());
pub static COMPONENT_CPU: Lazy<ValidName> =
    Lazy::new(|| ValidName::parse("cpu".to_owned()).unwrap());
pub static COMPONENT_MEM: Lazy<ValidName> =
    Lazy::new(|| ValidName::parse("memory".to_owned()).unwrap());

pub fn ensure_lazies() {
    let _ = KEY_PODNAME.force();
    let _ = KEY_NAMESPACE.force();
    let _ = KEY_STATUS.force();
    let _ = COMPONENT_CPU.force();
    let _ = COMPONENT_MEM.force();
}

// Replace by `std::sync::LazyLock` once it is stable
pub struct Lazy<T, F = fn() -> T> {
    cell: OnceLock<T>,
    init: Cell<Option<F>>,
}

// We never make a &F and OnceLock guarantees that there will
// only ever be one call of `init`
unsafe impl<T, F: Send> Sync for Lazy<T, F> where OnceLock<T>: Sync {}

impl<T, F: FnOnce() -> T> Lazy<T, F> {
    const fn new(f: F) -> Self {
        Self {
            cell: OnceLock::new(),
            init: Cell::new(Some(f)),
        }
    }

    fn force(&self) -> &T {
        self.cell.get_or_init(|| self.init.take().unwrap()())
    }
}

impl<T, F: FnOnce() -> T> Deref for Lazy<T, F> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.force()
    }
}

impl<T, F, U> AsRef<U> for Lazy<T, F>
where
    T: AsRef<U>,
    F: FnOnce() -> T,
    U: ?Sized,
{
    fn as_ref(&self) -> &U {
        self.deref().as_ref()
    }
}
