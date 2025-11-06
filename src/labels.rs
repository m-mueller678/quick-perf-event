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
            fn meta()->&'static [$crate::LabelMeta]{
                &const{[
                    $($crate::LabelMeta::new(stringify!($f)),)*
                ]}
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
    fn meta() -> &'static [LabelMeta];
    /// Calls `f` for each label value, in the same order as [`names`](Self::names).
    fn values(&self, f: &mut dyn FnMut(&str));
}

/// Metadata about a label
pub struct LabelMeta {
    name: &'static str,
    width: usize,
}

impl LabelMeta {
    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub const fn new(name: &'static str) -> Self {
        LabelMeta { name, width: 9 }
    }

    pub const fn with_width(mut self, width: usize) -> Self {
        self.width = width;
        self
    }
}

/// No labels.
impl Labels for () {
    fn meta() -> &'static [LabelMeta] {
        &[]
    }

    fn values(&self, _f: &mut dyn FnMut(&str)) {}
}

/// Treats the string as a single label with name `"label"`.
impl Labels for str {
    fn meta() -> &'static [LabelMeta] {
        &[LabelMeta {
            name: "label",
            width: 30,
        }]
    }

    fn values(&self, f: &mut dyn FnMut(&str)) {
        f(self)
    }
}
