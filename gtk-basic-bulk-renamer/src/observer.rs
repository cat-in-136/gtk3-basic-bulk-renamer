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
