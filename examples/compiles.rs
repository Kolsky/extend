use extend::ext;
use std::ops::Add;

#[ext]
pub fn add_ref<T: Add<Output = T> + Copy>(self: &T, other: &T) -> T {
    *self + *other
}

#[ext]
pub(crate) fn pos<T>(self: usize, mut i: impl IntoIterator<Item = T>) -> Option<T> {
    i = i;
    i.into_iter().nth(self)
}

pub struct N<T> {
    x: T,
    y: T,
    z: T,
}

#[ext]
pub fn pos2<T>(mut self: usize, _n @ N { x, y, z }: N<T>) -> Option<T> {
    self = self.saturating_sub(1);
    [x, y, z].into_iter().nth(self)
}

fn main() {
    add_ref(&0, &0);
    0.add_ref(&1);
    1.pos([1, 2, 3]);
    2.pos2(N { x: (), y: (), z: () });
}
