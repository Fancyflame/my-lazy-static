#![cfg_attr(feature = "no_std", no_std)]

use core::{
    cell::UnsafeCell,
    ops::Deref,
    sync::atomic::{AtomicBool, Ordering},
};

#[macro_export]
macro_rules! lazy_static {
    {
        $(
            $vis:vis static ref $name:ident:$ty:ty=$init:expr;
        )*
    } => {
        $(
            $vis static $name:$crate::Lazy<$ty>=$crate::Lazy{
                _cell:core::cell::UnsafeCell::new($crate::State::NotInitialized(&||$init)),
                _lock:core::sync::atomic::AtomicBool::new(false)
            };
        )*
    };
}

pub enum State<T: Sync + 'static> {
    Initialized(T),
    NotInitialized(&'static dyn Fn() -> T),
}

pub struct Lazy<T: Sync + 'static> {
    pub _cell: UnsafeCell<State<T>>,
    pub _lock: AtomicBool,
}

unsafe impl<T: Sync> Sync for Lazy<T> {}

impl<T: Sync> Deref for Lazy<T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe {
            let ptr = self._cell.get();
            match *ptr {
                State::Initialized(ref n) => n,
                State::NotInitialized(ref func) => {
                    if self._lock.swap(true, Ordering::Relaxed) {
                        #[cfg(feature = "no_std")]
                        panic!("This static variable is initializing, cannot initialize at the meantime.");
                        #[cfg(not(feature = "no_std"))]
                        {
                            std::thread::yield_now();
                            return self.deref();
                        }
                    }
                    *ptr = State::Initialized(func());
                    self.deref()
                }
            }
        }
    }
}

#[inline]
pub fn initialize<T: Sync>(t: Lazy<T>) {
    let _ = t.deref();
}

#[test]
#[cfg(not(feature = "no_std"))]
fn test() {
    lazy_static! {
        static ref TEST: String = "hello world".to_string();
        static ref TEST2: Vec<u8> = vec![1, 2, 3, 4];
    }

    let t1 = std::thread::spawn(|| {
        assert_eq!(*TEST, "hello world");
    });

    let t2 = std::thread::spawn(|| {
        assert_eq!(*TEST2, [1, 2, 3, 4]);
    });

    t1.join().unwrap();
    t2.join().unwrap();
}

#[test]
#[cfg(feature = "no_std")]
fn test2() {
    lazy_static! {
        static ref TEST: u32 = 500;
    }
    assert_eq!(*TEST, 500);
}
