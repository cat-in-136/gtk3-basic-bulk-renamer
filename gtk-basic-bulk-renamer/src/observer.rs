use std::cell::RefCell;
use std::rc::Rc;

pub(crate) trait Observer<T, E> {
    fn update(&self, arg: &T) -> Result<(), E>;
}

// pub(crate) struct FnObserver<T, E>(pub Box<dyn Fn(&T) -> Result<(), E>>);
//
// impl<T, E> Observer<T, E> for FnObserver<T, E> {
//     fn update(&self, arg: &T) -> Result<(), E> {
//         self.0(arg)
//     }
// }

pub(crate) struct SubjectImpl<T, E> {
    observers: RefCell<Vec<Rc<dyn Observer<T, E>>>>,
}

impl<T, E> SubjectImpl<T, E> {
    pub fn new() -> Self {
        Self {
            observers: RefCell::new(Vec::new()),
        }
    }

    pub fn attach(&self, observer: Rc<dyn Observer<T, E>>) {
        self.observers.borrow_mut().push(observer);
    }

    pub fn notify(&self, arg: T) -> Result<(), E> {
        for observer in self.observers.borrow_mut().iter_mut() {
            observer.update(&arg)?;
        }
        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod test {
    use super::*;
    use crate::error::Error;
    use core::sync::atomic::{AtomicUsize, Ordering};

    pub(crate) struct CounterObserver {
        count: Rc<RefCell<AtomicUsize>>,
    }

    impl CounterObserver {
        pub(crate) fn new() -> Self {
            Self {
                count: Rc::new(RefCell::new(AtomicUsize::new(0))),
            }
        }

        pub(crate) fn reset(&self) {
            let count = self.count.borrow_mut();
            count.store(0, Ordering::SeqCst);
        }

        pub(crate) fn count(&self) -> usize {
            let count = self.count.borrow();
            count.load(Ordering::SeqCst)
        }
    }

    impl Observer<(), Error> for CounterObserver {
        fn update(&self, arg: &()) -> Result<(), Error> {
            let count = self.count.borrow_mut();
            count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    #[test]
    fn test_subject_impl() {
        let subject = SubjectImpl::new();
        let observer = Rc::new(CounterObserver::new());

        subject.attach(observer.clone());
        assert_eq!(subject.observers.borrow().len(), 1);

        observer.reset();
        subject.notify(()).unwrap();
        assert_eq!(observer.count(), 1);
    }
}
