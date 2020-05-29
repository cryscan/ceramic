use serde::{Serialize, Deserialize};

pub trait Redirect<T, U> {
    fn redirect<F>(self, map: &F) -> Self
        where F: Fn(T) -> U;

    fn redirect_mut<F>(&mut self, map: &F)
        where F: Fn(T) -> U,
              Self: Sized + Copy {
        *self = self.redirect(map);
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RedirectItem<T, U> {
    Origin(T),
    Target(U),
}

impl<T, U> Redirect<T, U> for RedirectItem<T, U> {
    fn redirect<F>(self, map: &F) -> Self
        where F: Fn(T) -> U {
        match self {
            RedirectItem::Origin(origin) => RedirectItem::Target(map(origin)),
            RedirectItem::Target(target) => RedirectItem::Target(target),
        }
    }
}

impl<T, U> RedirectItem<T, U> {
    pub fn unwrap(self) -> U {
        match self {
            RedirectItem::Origin(_) => panic!("Item not redirected."),
            RedirectItem::Target(target) => target,
        }
    }

    pub fn iter<'a>(&'a self) -> Iter<'a, T, U>
        where U: Copy {
        Iter::<'a, T, U> { item: self, pos: 0 }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Iter<'a, T, U> {
    item: &'a RedirectItem<T, U>,
    pos: usize,
}

impl<T, U> Iter<'_, T, U> {
    const SIZE: usize = 1;
}

impl<T, U> Iterator for Iter<'_, T, U>
    where U: Copy {
    type Item = U;

    fn next(&mut self) -> Option<Self::Item> {
        let pos = self.pos;
        self.pos += 1;

        if (0..Self::SIZE).contains(&pos) {
            match self.item {
                RedirectItem::Origin(_) => None,
                RedirectItem::Target(target) => Some(*target),
            }
        } else {
            None
        }
    }
}

impl<T, U> DoubleEndedIterator for Iter<'_, T, U>
    where U: Copy {
    fn next_back(&mut self) -> Option<Self::Item> {
        let pos = self.pos;
        self.pos -= 1;

        if (0..Self::SIZE).contains(&pos) {
            match self.item {
                RedirectItem::Origin(_) => None,
                RedirectItem::Target(target) => Some(*target),
            }
        } else {
            None
        }
    }
}