use super::Format;
use crate::{
    counters::{CounterReading, Counters},
    visit,
};
use std::{
    error::Error,
    io::{Write, stdout},
    iter,
    time::UNIX_EPOCH,
};

pub struct Csv {
    header_written: bool,
    reading_buffer: Vec<CounterReading>,
    writer: csv::Writer<Box<dyn Write>>,
}

impl Csv {
    pub fn new() -> Self {
        Csv {
            header_written: false,
            reading_buffer: Vec::new(),
            writer: csv::Writer::from_writer(Box::new(stdout())),
        }
    }
}
impl Format for Csv {
    fn push(
        &mut self,
        scale: usize,
        start_time: std::time::SystemTime,
        counters: &mut dyn Counters,
        labels: &mut dyn FnMut(&mut dyn FnMut(&str)),
        label_names: &'static [&'static str],
    ) -> Result<(), Box<dyn Error>> {
        let mut err = Ok(());
        if !self.header_written {
            self.header_written = true;
            visit(label_names, &mut |x: &str| {
                if err.is_ok() {
                    err = self.writer.write_field(x)
                }
            });
            self.writer.write_field("start_time")?;
            self.writer.write_field("scale")?;
            counters.names(&mut |x| {
                if err.is_ok() {
                    err = self.writer.write_field(x)
                }
            });
            self.writer.write_field("multiplexed")?;
            self.writer.write_record(iter::empty::<&[u8]>())?;
        }
        labels(&mut |x| {
            if err.is_ok() {
                err = self.writer.write_field(x)
            }
        });
        err?;
        self.writer.write_field(
            &start_time
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs_f64()
                .to_string(),
        )?;
        self.reading_buffer.clear();
        counters.read(&mut self.reading_buffer);
        self.writer.write_field(&scale.to_string())?;
        let mut any_multiplexed = false;
        for reading in &self.reading_buffer {
            any_multiplexed |= reading.multiplexed;
            self.writer
                .write_field(reading.scaled_value(scale).to_string())?;
        }
        self.writer.write_field(any_multiplexed.to_string())?;
        self.writer.write_record(iter::empty::<&[u8]>())?;
        self.writer.flush()?;
        Ok(())
    }

    fn dump_and_reset(
        &mut self,
        _label_names: &'static [&'static str],
        _counters: &mut dyn Counters,
    ) -> Result<(), Box<dyn Error>> {
        self.header_written = false;
        Ok(())
    }
}
