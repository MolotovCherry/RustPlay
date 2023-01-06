use std::cell::{Ref, RefCell, RefMut};

use smallvec::SmallVec;

type InnerData<DataType, const N: usize> = SmallVec<[DataType; N]>;

/// A SmallVec Data wrapper used to transfer data, e.g. in trait impl's
/// where you cannot use return values to return data.
///
/// Since this uses RefCell, you can borrow mutably from a shared reference,
/// and this avoid some nasty multiple mutable borrowing problems
///
/// If you do not specify the size, it will be 1 by default.
///
/// This uses SmallVec in order to avoid creating needless allocations
pub struct Data<DataType, const N: usize = 1> {
    data: RefCell<InnerData<DataType, N>>,
}

impl<DataType, const N: usize> Data<DataType, N> {
    pub fn new() -> Self {
        Data {
            data: RefCell::new(SmallVec::new()),
        }
    }

    pub fn borrow(&self) -> Ref<InnerData<DataType, N>> {
        self.data.borrow()
    }

    pub fn borrow_mut(&self) -> RefMut<InnerData<DataType, N>> {
        self.data.borrow_mut()
    }

    pub fn get_mut(&mut self) -> &mut InnerData<DataType, N> {
        self.data.get_mut()
    }
}
