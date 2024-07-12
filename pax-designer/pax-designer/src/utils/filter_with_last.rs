pub struct FilterWithLast<I, F>
where
    I: Iterator,
    F: FnMut(&I::Item) -> bool,
{
    iter: I,
    predicate: F,
    last_item: Option<I::Item>,
}

impl<I, F> Iterator for FilterWithLast<I, F>
where
    I: Iterator,
    F: FnMut(&I::Item) -> bool,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(item) = self.iter.next() {
            if (self.predicate)(&item) {
                return Some(item);
            }
            self.last_item = Some(item);
        }
        self.last_item.take()
    }
}

pub trait FilterWithLastExt: Iterator {
    fn filter_with_last<F>(self, predicate: F) -> FilterWithLast<Self, F>
    where
        Self: Sized,
        F: FnMut(&Self::Item) -> bool,
    {
        FilterWithLast {
            iter: self,
            predicate,
            last_item: None,
        }
    }
}

impl<T: ?Sized> FilterWithLastExt for T where T: Iterator {}
