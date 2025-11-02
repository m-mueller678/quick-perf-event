#[macro_export]
macro_rules! struct_labels {
    ($vis:vis struct $Name:ident{
        $($fv:vis $f:ident:$F:ty,)*
    }) => {
        $vis struct $Name{
            $($fv $f:$F,)*
        }

        impl $crate::Labels for $Name{
            fn names()->&'static [&'static str]{
                &[
                    $(std::stringify!($f),)*
                ]
            }

            fn values(&self,f:&mut dyn FnMut(&str)){
                $(f(&self.$f);)*
            }
        }
    };
}

pub trait Labels {
    fn names() -> &'static [&'static str];
    fn values(&self, f: &mut dyn FnMut(&str));
}

impl Labels for () {
    fn names() -> &'static [&'static str] {
        &[]
    }

    fn values(&self, _f: &mut dyn FnMut(&str)) {}
}

impl Labels for str {
    fn names() -> &'static [&'static str] {
        &["label"]
    }

    fn values(&self, f: &mut dyn FnMut(&str)) {
        f(self)
    }
}
