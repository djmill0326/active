use std::{cell::UnsafeCell, collections::{HashMap, HashSet}, marker::PhantomData, mem::size_of};

// no error handling, unsafe everywhere. bad code. don't use

macro_rules! global {
    ($name: ident: $type: ident) => {
        static mut $name: Option<$type> = None;
    };
    ($name: ident: $type: ident = $default: expr) => {
        static mut $name: Option<$type> = Some($default);
    };
    ($name: ident) => {
        unsafe { $name.as_mut().unwrap_unchecked() }
    };
    ($name: ident = $value: expr) => {
        unsafe { $name = Some($value) }
    };
}

global!(LISTENERS: Listeners);
global!(LISTENER_TAG: usize = 0);
global!(STORAGE: FakeStorage);

pub type Resolver<T> = fn(x: T) -> T;
pub type Listener<T> = fn(x: &T) -> ();

pub trait Active<T> {
    fn cmp(&self, x: &T) -> bool;
    fn update(&mut self, x: T) -> bool;
    fn listen(&mut self, f: Listener<T>) -> bool;
    fn unlisten(&mut self, f: Listener<T>) -> bool;
    fn resolve(&self) -> &T;
}

#[derive(Clone, Copy)]
struct ValueShell<T: Clone + Copy> (T, Option<T>, Resolver<T>, usize);

pub struct Value<T: Clone + Copy> {
    data: T,
    cache: UnsafeCell<Option<T>>,
    resolver: Resolver<T>,
    listener_tag: usize
}

type Listeners = HashMap<usize, HashSet<Listener<usize>>>;

fn get_listeners<'a, T>(tag: usize) -> &'a HashSet<Listener<T>> {
    // ok listen, i know what you're thinking. i don't give a fuck. void* in Rust
    unsafe { std::mem::transmute(global!(LISTENERS).get(&tag).unwrap_unchecked()) }
}

fn get_listeners_mut<'a, T>(tag: usize) -> &'a mut HashSet<Listener<T>> {
    unsafe { std::mem::transmute(global!(LISTENERS).get_mut(&tag).unwrap_unchecked()) }
}

fn register_listener_tag() -> usize {
    let tag = global!(LISTENER_TAG);
    global!(LISTENER_TAG = *tag + 1);
    global!(LISTENERS).insert(*tag, HashSet::new());
    *tag
}

impl<T: Clone + Copy> Value<T> {
    unsafe fn get_cache(&self) -> &mut Option<T> {
        // i don't even care anymore. unsound interior mutability
        std::mem::transmute(self.cache.get())
    }

    fn update_cache(&self, x: T) {
        let cache = unsafe { self.get_cache() };
        *cache = Some(x);
    }

    fn get_cached(&self) -> Option<&T> {
        unsafe { self.get_cache().as_ref() }
    }

    pub fn is_cached(&self) -> bool {
        unsafe { self.get_cache().is_some() }
    }

    fn dirty(&mut self) {
        let cache = unsafe { self.get_cache() };
        *cache = None;
    }

    pub fn new(x: T, transform: fn (x: T) -> T) -> Self {
        Self {
            data: x,
            cache: UnsafeCell::new(None),
            resolver: transform,
            listener_tag: register_listener_tag()
        }
    }
}

impl<T: Clone + Copy + PartialEq> Active<T> for Value<T> {
    fn cmp(&self, x: &T) -> bool {
        self.data == *x
    }

    fn update(&mut self, x: T) -> bool {
        if self.cmp(&x) { false }
        else {
            self.data = x;
            self.dirty();
            get_listeners(self.listener_tag).iter().for_each(|callback| callback(self.resolve()));
            true
        }
    }

    fn listen(&mut self, f: Listener<T>) -> bool {
        get_listeners_mut(self.listener_tag).insert(f)
    }

    fn unlisten(&mut self, f: Listener<T>) -> bool {
        get_listeners_mut(self.listener_tag).remove(&f)
    }

    fn resolve(&self) -> &T {
        if !self.is_cached() {
            let new_value = (self.resolver)(self.data);
            self.update_cache(new_value)
        }
        unsafe { self.get_cached().unwrap_unchecked() }
    }
}

struct EvilVec<'a, T> {
    storage: &'a mut Vec<u8>,
    _phantom_data: PhantomData<T>
}

impl<'a, T> EvilVec<'a, T> {
    fn get<Type>(&self, index: usize) -> &'a Type {
        assert!(size_of::<Type>() <= size_of::<T>(), "what the hell dude. i cannot fit that.");
        unsafe { std::mem::transmute(&self.storage[index * size_of::<T>()]) }
    }

    fn get_mut<Type>(&mut self, index: usize) -> &'a mut Type {
        assert!(size_of::<Type>() <= size_of::<T>(), "what the hell dude. i cannot fit that.");
        unsafe { std::mem::transmute(&mut self.storage[index * size_of::<T>()]) }
    }

    fn store<Type>(&mut self, value: Type) -> usize {
        assert!(size_of::<Type>() <= size_of::<T>(), "what the hell dude. i cannot fit that.");
        let index = self.storage.len();
        self.storage.resize(index + size_of::<T>(), 0);
        let slice: &mut Type = unsafe { std::mem::transmute(&mut self.storage[index]) };
        *slice = value;
        index
    }
}

struct Slab<'a, Alloc> {
    storage: EvilVec<'a, Alloc>
}

impl<'a, T: Copy> Slab<'a, T> {
    // todo: make it possible to delete. this is a bump allocator i guess

    fn get<Type>(&self, index: usize) -> &'a Type {
        self.storage.get(index)
    }

    fn get_mut<Type>(&mut self, index: usize) -> &'a mut Type {
        self.storage.get_mut(index)
    }

    fn store<Type>(&mut self, value: Type) -> usize {
        self.storage.store(value)
    }

    fn update<Type: Copy + 'a>(&mut self, index: usize, value: &Type) {
        let reference: &mut Type = self.get_mut::<Type>(index);
        *reference = *value;
    }

    fn new(storage: &'a mut Vec<u8>) -> Self {
        Self {
            storage: EvilVec {
                storage: storage,
                _phantom_data: PhantomData
            }
        }
    }
}

type FakeStorage = Vec<Vec<u8>>;

pub struct Variable<T: Clone + Copy> {
    tag: usize,
    _phantom_value: PhantomData<Value<T>>
}

impl<T: Clone + Copy + PartialEq> Variable<T> {
    fn get_slab<'a>() -> Slab<'a, ValueShell<T>> {
        Slab::new(&mut global!(STORAGE)[size_of::<Value<T>>()])
    }

    pub fn get<'a>(&self) -> &'a Value<T> {
        Self::get_slab().get::<Value<T>>(self.tag)
    }

    pub fn get_mut<'a>(&self) -> &'a mut Value<T> {
        Self::get_slab().get_mut::<Value<T>>(self.tag)
    }

    pub fn transformed(value: T, resolver: Resolver<T>) -> Self {
        let tag = Self::get_slab().store(Value::new(value, resolver));
        Self {
            tag: tag,
            _phantom_value: PhantomData
        }
    }

    pub fn new(value: T) -> Self {
        Self::transformed(value, |x| x)
    }
}

impl<T: Clone + Copy + PartialEq> Active<T> for Variable<T> {
    fn cmp(&self, x: &T) -> bool {
        self.get().cmp(x)
    }

    fn update(&mut self, x: T) -> bool {
        self.get_mut().update(x)
    }

    fn listen(&mut self, f: Listener<T>) -> bool {
        self.get_mut().listen(f)
    }

    fn unlisten(&mut self, f: Listener<T>) -> bool {
        self.get_mut().unlisten(f)
    }

    fn resolve(&self) -> &T {
        self.get().resolve()
    }
}

pub type Int = Variable<i64>;
pub type Number = Variable<f64>;

pub fn init_world() {
    global!(STORAGE = Vec::new());
    global!(STORAGE).resize(64, Vec::new());
    global!(LISTENERS = HashMap::new());
}