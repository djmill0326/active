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