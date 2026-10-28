//! The Tasks tab's history: the last [`SPAN`] samples of the device's CPU, memory and load, and
//! of each process's CPU and memory, one sample a frame (a second), for the graphs a task manager
//! draws. Bounded: a series keeps [`SPAN`] samples, and a process's series go when the process
//! does, so a window left open for days holds what it held after two minutes (T1-046).

use std::collections::{HashMap, VecDeque};

use crate::stats::Frame;

/// Samples kept: two minutes at one a second.
pub const SPAN: usize = 120;

/// The last [`SPAN`] values of one variable, oldest first.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Series(VecDeque<f32>);

impl Series {
    pub fn push(&mut self, v: f32) {
        if self.0.len() == SPAN {
            self.0.pop_front();
        }
        self.0.push_back(v);
    }

    pub fn values(&self) -> Vec<f32> {
        self.0.iter().copied().collect()
    }

    pub fn last(&self) -> Option<f32> {
        self.0.back().copied()
    }

    pub fn max(&self) -> f32 {
        self.0.iter().copied().fold(0.0, f32::max)
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

/// A process's series.
#[derive(Clone, Debug, Default)]
pub struct ProcessHistory {
    pub cpu: Series,
    pub mem: Series,
}

#[derive(Clone, Debug, Default)]
pub struct History {
    pub cpu: Series,
    pub mem: Series,
    pub load: Series,
    /// The simulated battery: its level (%) and the power through it (W, + charging).
    pub battery: Series,
    pub power: Series,
    pub mem_total: f32,
    pub processes: HashMap<u32, ProcessHistory>,
}

impl History {
    /// One frame's sample; processes no longer in the frame are forgotten (after the device
    /// restarts, every process is another).
    pub fn record(&mut self, f: &Frame) {
        self.cpu.push(f.cpu_percent);
        self.mem.push(f.mem_total_mib - f.mem_available_mib);
        self.load.push(f.load[0]);
        if let Some(p) = &f.power {
            self.battery.push(p.capacity_percent);
            self.power.push(p.watts());
        }
        self.mem_total = f.mem_total_mib;
        self.processes
            .retain(|pid, _| f.processes.iter().any(|p| p.pid == *pid));
        for p in &f.processes {
            let h = self.processes.entry(p.pid).or_default();
            h.cpu.push(p.cpu_percent);
            h.mem.push(p.resident_mib);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::Process;

    fn frame(pids: &[u32], cpu: f32) -> Frame {
        Frame {
            cpu_percent: cpu,
            mem_total_mib: 1000.0,
            mem_available_mib: 600.0,
            processes: pids
                .iter()
                .map(|&pid| Process {
                    pid,
                    cpu_percent: cpu,
                    resident_mib: 10.0,
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        }
    }

    #[test]
    fn series_keep_the_span_and_processes_go_with_their_process() {
        let mut h = History::default();
        for i in 0..(SPAN + 30) {
            h.record(&frame(&[1, 2], i as f32));
        }
        assert_eq!(h.cpu.len(), SPAN);
        assert_eq!(h.cpu.values()[0], 30.0, "the oldest sample dropped first");
        assert_eq!(h.mem.last(), Some(400.0));
        assert_eq!(h.processes[&2].cpu.len(), SPAN);
        h.record(&frame(&[1, 3], 0.0));
        assert!(
            !h.processes.contains_key(&2),
            "an ended process is forgotten"
        );
        assert_eq!(h.processes[&3].cpu.len(), 1);
        assert_eq!(h.processes.len(), 2);
    }
}
