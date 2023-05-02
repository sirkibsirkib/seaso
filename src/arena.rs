use core::marker::PhantomData;

type Index = u16;

struct Key<T> {
    index: Index,
    _phantom: PhantomData<T>,
}

struct Arena<T> {
    data: Vec<T>,
}

struct ArrayArena<T> {
    data: Vec<T>,
    array_len: Index,
}
