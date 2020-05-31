use serde::{Serialize, Deserialize};

pub trait Redirect<T, U> {
    fn redirect<F>(self, map: &F) -> Self
        where F: Fn(T) -> U;
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RedirectField<T, U> {
    Origin(T),
    Target(U),
}

impl<T, U> Redirect<T, U> for RedirectField<T, U> {
    fn redirect<F>(self, map: &F) -> Self
        where F: Fn(T) -> U {
        match self {
            RedirectField::Origin(origin) => RedirectField::Target(map(origin)),
            RedirectField::Target(target) => RedirectField::Target(target),
        }
    }
}

impl<T, U> RedirectField<T, U> {
    pub fn unwrap(self) -> U {
        match self {
            RedirectField::Origin(_) => panic!("Item not redirected."),
            RedirectField::Target(target) => target,
        }
    }

    pub fn iter<'a>(&'a self) -> Iter<'a, T, U>
        where {
        Iter::<'a, T, U> { item: self, pos: 0 }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Iter<'a, T, U> {
    item: &'a RedirectField<T, U>,
    pos: usize,
}

impl<T, U> Iter<'_, T, U> {
    const SIZE: usize = 1;
}

impl<'a, T, U> Iterator for Iter<'a, T, U> {
    type Item = &'a U;

    fn next(&mut self) -> Option<Self::Item> {
        let pos = self.pos;
        self.pos += 1;

        if (0..Self::SIZE).contains(&pos) {
            match self.item {
                RedirectField::Origin(_) => None,
                RedirectField::Target(target) => Some(target),
            }
        } else {
            None
        }
    }
}

impl<'a, T, U> DoubleEndedIterator for Iter<'a, T, U> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let pos = self.pos;
        self.pos -= 1;

        if (0..Self::SIZE).contains(&pos) {
            match self.item {
                RedirectField::Origin(_) => None,
                RedirectField::Target(target) => Some(target),
            }
        } else {
            None
        }
    }
}