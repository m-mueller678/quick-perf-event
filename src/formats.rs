mod csv;
mod live;
mod live_table;
mod tabled;
mod tabled_float;

pub use csv::Csv;
pub use live::Live;
pub use live_table::LiveTable;
pub use tabled::Tabled;
pub use tabled_float::TabledFloat;

use crate::{counters::Counters, labels::LabelMeta};
use std::error::Error;

pub trait Format {
    fn push(
        &mut self,
        scale: usize,
        start_time: std::time::SystemTime,
        counters: &mut dyn Counters,
        labels: &mut dyn FnMut(&mut dyn FnMut(&str)),
        label_meta: &'static [LabelMeta],
    ) -> Result<(), Box<dyn Error>>;
    fn dump_and_reset(
        &mut self,
        label_meta: &'static [LabelMeta],
        counters: &mut dyn Counters,
    ) -> Result<(), Box<dyn Error>>;
}

impl Format for Box<dyn Format> {
    fn push(
        &mut self,
        scale: usize,
        start_time: std::time::SystemTime,
        counters: &mut dyn Counters,
        labels: &mut dyn FnMut(&mut dyn FnMut(&str)),
        label_meta: &'static [LabelMeta],
    ) -> Result<(), Box<dyn Error>> {
        (**self).push(scale, start_time, counters, labels, label_meta)
    }

    fn dump_and_reset(
        &mut self,
        label_meta: &'static [LabelMeta],
        counters: &mut dyn Counters,
    ) -> Result<(), Box<dyn Error>> {
        (**self).dump_and_reset(label_meta, counters)
    }
}

pub fn format_from_env() -> Box<dyn Format> {
    match std::env::var("QPE_FORMAT").as_deref() {
        Ok("csv") => Box::new(Csv::new()),
        Ok("md") => Box::new(Tabled::new()),
        x => {
            match x {
                Ok(requested) => {
                    eprintln!(
                        "unrecognized value for QPE_FORMAT: {requested:?}.\nSupported values: csv, md"
                    );
                }
                Err(_) => {}
            }
            Box::new(Live::new())
        }
    }
}
