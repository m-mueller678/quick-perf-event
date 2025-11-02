/// Declares a struct type implementing the [`Labels`] trait.
///
/// This macro simplifies creating label structs for use with [`QuickPerfEvent`](crate::QuickPerfEvent).
/// It generates both the struct definition and its corresponding [`Labels`]
/// implementation. The fields must implement [`AsRef<str>`](std::convert::AsRef).
///
/// ```
#[doc = include_str!("../examples/struct_labels.rs")]
/// ```
#[macro_export]
macro_rules! struct_labels {
    ($vis:vis struct $Name:ident{
        $($fv:vis $f:ident:$F:ty,)* $(,)?
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
                $(f(std::convert::AsRef::as_ref(&self.$f));)*
            }
        }
    };
}

/// A trait for sets of labels attached to a performance measurement.
///
/// Implementors describe both the schema (via [`names`](Self::names))
/// and values (via [`values`](Self::values)) of a label set.
/// You can define a label struct conveniently using the
/// [`struct_labels!`](crate::struct_labels) macro.
pub trait Labels {
    /// Returns the static list of label names in order.
    fn names() -> &'static [&'static str];
    /// Calls `f` for each label value, in the same order as [`names`](Self::names).
    fn values(&self, f: &mut dyn FnMut(&str));
}

/// No labels.
impl Labels for () {
    fn names() -> &'static [&'static str] {
        &[]
    }

    fn values(&self, _f: &mut dyn FnMut(&str)) {}
}

/// Treats the string as a single label with name `"label"`.
impl Labels for str {
    fn names() -> &'static [&'static str] {
        &["label"]
    }

    fn values(&self, f: &mut dyn FnMut(&str)) {
        f(self)
    }
}
