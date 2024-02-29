macro_rules! global {
    ($name: ident: $type: ident) => {
        static mut $name: Option<$type> = None;
    };
    ($name: ident: $type: ident = $default: expr) => {
        static mut $name: Option<$type> = Some($default);
    };
    ($name: ident) => {
        unsafe {
            $name.as_mut().unwrap_unchecked()
        }
    };
    ($name: ident, $default: expr) => {
        unsafe {
            if let Some(x) = &mut $name {
                x
            } else {
                $name = Some($default);
                $name.as_mut().unwrap_unchecked()
            }
        }
    };
    ($name: ident = $value: expr) => {
        unsafe { $name = Some($value) }
    };
}

// other global impl if you want

static mut STORAGE: Vec<Vec<u8>> = Vec::new();

fn magic<T>(storage: &mut Vec<u8>, data: &T) {
    unsafe {
        let buf: &[u8] = std::mem::transmute([data, std::mem::transmute(size_of::<T>())]);
        storage.extend_from_slice(buf);
    }
}

fn globalize<T>(x: T) -> usize {
    unsafe {
        let index = STORAGE.len();
        STORAGE.push(Vec::new());
        magic(&mut STORAGE[index], &x);
        index
    }
}

fn update_global<T>(index: usize, x: T) {
    unsafe {
        STORAGE[index].clear();
        magic(&mut STORAGE[index], &x);
    }
}

fn get_global<'a, T>(index: usize) -> &'a T {
    unsafe { std::mem::transmute(STORAGE[index].as_ptr()) }
}