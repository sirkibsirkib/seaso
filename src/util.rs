use core::fmt::{Debug, Formatter, Result as FmtResult};

/// Newtype that suppresses pretty-printing of the wrapped type.
/// Useful in avoiding excessive indentation of internals when pretty printing its container.
pub(crate) struct NoPretty<T: Debug>(pub T);

/// Structure used in debug printing. Prints elements separated by commas.
pub(crate) struct CommaSep<'a, T: Debug + 'a, I: IntoIterator<Item = &'a T> + Clone> {
    pub iter: I,
    pub spaced: bool,
}

// pub trait Resettable: Sized {
//     fn reset(&mut self);
//     fn use_then_reset(&mut self) -> NotReset<Self> {
//         NotReset { t: self }
//     }
// }

// pub struct NotReset<'a, T: Resettable> {
//     pub t: &'a mut T,
// }

// impl<T: Resettable> Drop for NotReset<'_, T> {
//     fn drop(&mut self) {
//         self.t.reset();
//     }
// }

impl<T: Debug> Debug for NoPretty<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", &self.0)
    }
}

impl<'a, T: Debug + 'a, I: IntoIterator<Item = &'a T> + Clone> Debug for CommaSep<'a, T, I> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        for (i, x) in self.iter.clone().into_iter().enumerate() {
            if i > 0 {
                write!(f, "{}", if self.spaced { ", " } else { "," })?;
            }
            write!(f, "{:?}", x)?;
        }
        Ok(())
    }
}
