pub trait Redirect<T, U> {
    fn redirect<F>(self, map: &F) -> Self
        where F: Fn(T) -> U;
}

impl<T, U, V> Redirect<T, U> for Vec<V>
    where V: Redirect<T, U> {
    fn redirect<F>(self, map: &F) -> Self
        where F: Fn(T) -> U {
        self.into_iter().map(|v| v.redirect(map)).collect()
    }
}

impl<T, U, V> Redirect<T, U> for Option<V>
    where V: Redirect<T, U> {
    fn redirect<F>(self, map: &F) -> Self
        where F: Fn(T) -> U {
        self.map(|v| v.redirect(map))
    }
}

impl<T, U, V, E> Redirect<T, U> for Result<V, E>
    where V: Redirect<T, U> {
    fn redirect<F>(self, map: &F) -> Self
        where F: Fn(T) -> U {
        self.map(|v| v.redirect(map))
    }
}