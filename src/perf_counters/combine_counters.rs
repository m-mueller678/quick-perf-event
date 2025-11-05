use crate::counters::CounterBackend;

/// A [CounterBackend] that combines the counters from multiple backends.
///
/// You can construct this type via [`FromIterator`].
#[derive(Default)]
pub struct CombineCounters {
    backends: Vec<Box<dyn CounterBackend>>,
}

impl FromIterator<Box<dyn CounterBackend>> for CombineCounters {
    fn from_iter<T: IntoIterator<Item = Box<dyn CounterBackend>>>(iter: T) -> Self {
        CombineCounters {
            backends: iter.into_iter().collect(),
        }
    }
}

impl CounterBackend for CombineCounters {
    fn enable(&mut self) {
        for x in &mut self.backends {
            x.enable();
        }
    }

    fn disable(&mut self) {
        for x in &mut self.backends {
            x.disable();
        }
    }

    fn reset(&mut self) {
        for x in &mut self.backends {
            x.reset();
        }
    }

    fn read(&mut self, dst: &mut Vec<super::CounterReading>) {
        for x in &mut self.backends {
            x.read(dst);
        }
    }

    fn names(&self, dst: &mut dyn FnMut(&str)) {
        for x in &mut self.backends {
            x.names(dsr);
        }
    }
}
