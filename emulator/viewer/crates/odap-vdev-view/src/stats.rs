//! The device's load and processes, for the Instruments and Tasks tabs: the CPU, the memory and
//! every process, as `top` sees them on the device, once a second while a tab shows them; and the
//! actions a task manager takes on a process (signals, priority).
//!
//! One SSH session runs `top` in batch mode and streams its frames; nothing polls. The session
//! starts when the tab shows the system's group and ends when it hides it, so a window that does
//! not look costs the device nothing. Nothing is kept but the last frame.

use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

/// A process's line.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Process {
    pub pid: u32,
    pub user: String,
    pub nice: i32,
    /// top's state letter: R running, S sleeping, D waiting on the disk, T stopped, Z zombie,
    /// I idle (a kernel thread).
    pub state: char,
    pub name: String,
    pub cpu_percent: f32,
    pub resident_mib: f32,
    /// A kernel thread (no memory of its own): nothing a task manager acts on.
    pub kernel: bool,
}

/// The simulated hardware's power: the virtual board's battery model (the kernel's `test_power`,
/// which stands for the PMIC and the fuel gauge). Its values are in the model's own units
/// (millivolts, milliamperes, degrees), whatever the power_supply ABI says.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Power {
    pub capacity_percent: f32,
    pub status: String,
    pub health: String,
    pub technology: String,
    pub voltage_v: f32,
    /// Positive while charging, negative while discharging.
    pub current_a: f32,
    pub temperature_c: f32,
    pub time_to_empty_s: Option<u32>,
    pub time_to_full_s: Option<u32>,
    pub usb_online: bool,
    pub ac_online: bool,
}

impl Power {
    /// Volts times amperes: what flows into (+) or out of (−) the battery.
    pub fn watts(&self) -> f32 {
        self.voltage_v * self.current_a
    }
}

/// One `@power` line: `KEY=VALUE` pairs from the battery's uevent, then `usb=` and `ac=`.
pub fn parse_power(line: &str) -> Option<Power> {
    let rest = line.strip_prefix("@power ")?;
    let mut p = Power::default();
    let mut seen = false;
    for pair in rest.split_whitespace() {
        let Some((k, v)) = pair.split_once('=') else {
            continue;
        };
        let num = || v.parse::<f32>().ok();
        match k {
            "POWER_SUPPLY_CAPACITY" => {
                p.capacity_percent = num()?;
                seen = true;
            }
            "POWER_SUPPLY_STATUS" => p.status = v.to_string(),
            "POWER_SUPPLY_HEALTH" => p.health = v.to_string(),
            "POWER_SUPPLY_TECHNOLOGY" => p.technology = v.to_string(),
            "POWER_SUPPLY_VOLTAGE_NOW" => p.voltage_v = num()? / 1000.0,
            "POWER_SUPPLY_CURRENT_NOW" => p.current_a = num()? / 1000.0,
            "POWER_SUPPLY_TEMP" => p.temperature_c = num()?,
            "POWER_SUPPLY_TIME_TO_EMPTY_NOW" => p.time_to_empty_s = v.parse().ok(),
            "POWER_SUPPLY_TIME_TO_FULL_NOW" => p.time_to_full_s = v.parse().ok(),
            "usb" => p.usb_online = v == "1",
            "ac" => p.ac_online = v == "1",
            _ => {}
        }
    }
    seen.then_some(p)
}

/// One `top` frame.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Frame {
    /// The simulated hardware's power at the frame's time, when the board has a battery model.
    pub power: Option<Power>,
    /// Of all the CPUs together (100 % = all busy).
    pub cpu_percent: f32,
    pub load: [f32; 3],
    pub mem_total_mib: f32,
    pub mem_used_mib: f32,
    pub mem_available_mib: f32,
    pub tasks: u32,
    pub uptime: String,
    pub processes: Vec<Process>,
}

/// A size as `top` prints it in KiB, with its own suffixes past that (m, g, t).
fn kib_to_mib(s: &str) -> Option<f32> {
    let (n, mult) = match s.chars().last()? {
        'm' => (&s[..s.len() - 1], 1.0),
        'g' => (&s[..s.len() - 1], 1024.0),
        't' => (&s[..s.len() - 1], 1024.0 * 1024.0),
        _ => (s, 1.0 / 1024.0),
    };
    n.parse::<f32>().ok().map(|v| v * mult)
}

/// The number before `label` in a summary line (`3903.4 total`).
fn before(line: &str, label: &str) -> Option<f32> {
    let words: Vec<&str> = line
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|w| !w.is_empty())
        .collect();
    words
        .windows(2)
        .find(|w| w[1].starts_with(label))
        .and_then(|w| w[0].parse().ok())
}

/// Parses one frame's lines (from `top - …` to the end of its process table).
pub fn parse(lines: &[String]) -> Option<Frame> {
    let mut f = Frame::default();
    let mut table = false;
    for line in lines {
        if let Some(rest) = line.strip_prefix("top - ") {
            let up = rest.split(" up ").nth(1)?;
            f.uptime = up
                .split(",  ")
                .next()
                .unwrap_or("")
                .trim()
                .trim_end_matches(',')
                .to_string();
            let load = rest.split("load average:").nth(1)?;
            let v: Vec<f32> = load
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect();
            f.load = [*v.first()?, *v.get(1)?, *v.get(2)?];
        } else if line.starts_with("Tasks:") {
            f.tasks = before(line, "total").unwrap_or(0.0) as u32;
        } else if line.starts_with("%Cpu") {
            // "100.0 id" may lose its space after the comma: split on commas first.
            let idle = line.split(',').find(|p| p.trim_end().ends_with("id"))?;
            let idle: f32 = idle
                .trim()
                .trim_end_matches("id")
                .trim()
                .trim_start_matches(|c: char| !c.is_ascii_digit())
                .parse()
                .ok()?;
            f.cpu_percent = (100.0 - idle).max(0.0);
        } else if line.starts_with("MiB Mem") {
            f.mem_total_mib = before(line, "total")?;
            f.mem_used_mib = before(line, "used")?;
        } else if line.starts_with("MiB Swap") {
            f.mem_available_mib = before(line, "avail").unwrap_or(0.0);
        } else if line.trim_start().starts_with("PID ") {
            table = true;
        } else if table {
            let w: Vec<&str> = line.split_whitespace().collect();
            if w.len() < 12 {
                continue;
            }
            f.processes.push(Process {
                pid: w[0].parse().unwrap_or(0),
                user: w[1].to_string(),
                nice: w[3].parse().unwrap_or(0),
                state: w[7].chars().next().unwrap_or('?'),
                name: w[11..].join(" "),
                cpu_percent: w[8].parse().unwrap_or(0.0),
                resident_mib: kib_to_mib(w[5]).unwrap_or(0.0),
                kernel: w[4] == "0",
            });
        }
    }
    (f.mem_total_mib > 0.0).then_some(f)
}

/// The user's whole name: `top` cuts a name longer than its column to its first characters and a `+`
/// (the session user `blackbird` shows as `blackbi+`), so a cut name becomes the one user of the
/// device it can be; with none or several, it stays as `top` shows it.
pub fn full_user(shown: &str, users: &[String]) -> String {
    let Some(start) = shown.strip_suffix('+') else {
        return shown.to_string();
    };
    let mut fits = users
        .iter()
        .filter(|u| u.len() > start.len() && u.starts_with(start));
    match (fits.next(), fits.next()) {
        (Some(u), None) => u.clone(),
        _ => shown.to_string(),
    }
}

/// Follows the device's `top` while it runs; dropping it ends the session.
pub struct Watch {
    child: Arc<Mutex<Option<Child>>>,
}

impl Watch {
    /// `on_frame` gets each frame, on the watch's thread.
    pub fn start(vd_dir: PathBuf, on_frame: impl Fn(Frame) + Send + 'static) -> Watch {
        let child = Command::new(vd_dir.join("bin/odap-vdev"))
            // Every process, so the Tasks tab can sort and act on any (about 10 kB a second, only
            // while a tab shows them); and, between frames, the simulated hardware's power once a
            // second (`@power` lines, the battery model's uevent and its chargers).
            .args([
                "ssh",
                "--",
                "echo \"@users $(cut -d: -f1 /etc/passwd | tr '\\n' ' ')\"; \
                 b=/sys/class/power_supply/test_battery; \
                 (while [ -r $b/uevent ]; do echo \"@power $(tr '\\n' ' ' < $b/uevent) \
                 usb=$(cat /sys/class/power_supply/test_usb/online 2>/dev/null) \
                 ac=$(cat /sys/class/power_supply/test_ac/online 2>/dev/null)\"; sleep 1; done) & \
                 exec top -b -d 1 -w 400 -o %CPU",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok();
        let child = Arc::new(Mutex::new(child));
        let out = child
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .as_mut()
            .and_then(|c| c.stdout.take());
        if let Some(out) = out {
            std::thread::spawn(move || {
                let mut lines: Vec<String> = Vec::new();
                let mut first = true;
                let mut power = None;
                let mut users: Vec<String> = Vec::new();
                for line in BufReader::new(out).lines().map_while(Result::ok) {
                    if line.starts_with("@power ") {
                        power = parse_power(&line);
                        continue;
                    }
                    if let Some(names) = line.strip_prefix("@users ") {
                        users = names.split_whitespace().map(String::from).collect();
                        continue;
                    }
                    if line.starts_with("top - ") && !lines.is_empty() {
                        // The first frame's %CPU counts since boot: skipped.
                        if !first && let Some(mut f) = parse(&lines) {
                            f.power = power.clone();
                            for p in &mut f.processes {
                                p.user = full_user(&p.user, &users);
                            }
                            on_frame(f);
                        }
                        first = false;
                        lines.clear();
                    }
                    lines.push(line);
                }
            });
        }
        Watch { child }
    }
}

impl Drop for Watch {
    fn drop(&mut self) {
        if let Some(mut c) = self.child.lock().unwrap_or_else(|p| p.into_inner()).take() {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}

/// What a task manager does to a process.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// SIGTERM: asks it to end.
    End,
    /// SIGKILL: ends it at once.
    Kill,
    /// SIGSTOP and SIGCONT.
    Pause,
    Resume,
    /// Its niceness one step up (lower priority) or down (higher).
    Lower,
    Raise,
}

impl Action {
    /// The shell command on the device.
    pub fn command(self, p: &Process) -> String {
        match self {
            Action::End => format!("kill -TERM {}", p.pid),
            Action::Kill => format!("kill -KILL {}", p.pid),
            Action::Pause => format!("kill -STOP {}", p.pid),
            Action::Resume => format!("kill -CONT {}", p.pid),
            Action::Lower => format!("renice -n {} -p {}", (p.nice + 1).min(19), p.pid),
            Action::Raise => format!("renice -n {} -p {}", (p.nice - 1).max(-20), p.pid),
        }
    }

    /// Why it cannot act on `p`, if it cannot: never on init or a kernel thread.
    pub fn refused(self, p: &Process) -> Option<&'static str> {
        if p.pid == 1 {
            Some("init: ending it would stop the device")
        } else if p.kernel {
            Some("a kernel thread")
        } else {
            None
        }
    }
}

/// Runs `action` on the device; the error, if it failed.
pub fn act(vd_dir: &std::path::Path, action: Action, p: &Process) -> Result<(), String> {
    if let Some(why) = action.refused(p) {
        return Err(why.into());
    }
    let out = Command::new(vd_dir.join("bin/odap-vdev"))
        .args(["ssh", "--", &action.command(p)])
        .stdin(Stdio::null())
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAME: &str = "top - 07:49:54 up  2:02,  2 users,  load average: 0.01, 0.20, 1.50
Tasks: 104 total,   1 running, 103 sleeping,   0 stopped,   0 zombie
%Cpu(s): 10.5 us,  2.0 sy,  0.0 ni, 87.5 id,  0.0 wa,  0.0 hi,  0.0 si,  0.0 st
MiB Mem :   3903.4 total,   3302.3 free,    403.1 used,    333.3 buff/cache
MiB Swap:      0.0 total,      0.0 free,      0.0 used.   3500.3 avail Mem

    PID USER      PR  NI    VIRT    RES    SHR S  %CPU  %MEM     TIME+ COMMAND
    812 blackbi+  20   0  900000  81240  40000 S  12.3   2.0   1:02.45 composer
    901 blackbi+  20   0  700000   1.2g  30000 S   3.0   1.0   0:10.00 testbed
      1 root      20   0   23900  12640   9280 S   0.0   0.3   0:00.45 systemd";

    fn lines(s: &str) -> Vec<String> {
        s.lines().map(String::from).collect()
    }

    #[test]
    fn a_cut_user_name_is_the_one_user_it_can_be() {
        let users: Vec<String> = ["root", "blackbird", "messagebus"].map(String::from).into();
        assert_eq!(full_user("blackbi+", &users), "blackbird");
        assert_eq!(full_user("root", &users), "root");
        assert_eq!(full_user("nobody+", &users), "nobody+");
        let twins: Vec<String> = ["blackbird", "blackbirds"].map(String::from).into();
        assert_eq!(full_user("blackbi+", &twins), "blackbi+");
    }

    #[test]
    fn a_frame_parses() {
        let f = parse(&lines(FRAME)).unwrap();
        assert_eq!(f.load, [0.01, 0.20, 1.50]);
        assert_eq!(f.tasks, 104);
        assert!((f.cpu_percent - 12.5).abs() < 1e-4);
        assert_eq!(
            (f.mem_total_mib, f.mem_used_mib, f.mem_available_mib),
            (3903.4, 403.1, 3500.3)
        );
        assert_eq!(f.uptime, "2:02");
        assert_eq!(f.processes.len(), 3);
        assert_eq!(
            f.processes[0],
            Process {
                pid: 812,
                user: "blackbi+".into(),
                nice: 0,
                state: 'S',
                name: "composer".into(),
                cpu_percent: 12.3,
                resident_mib: 81240.0 / 1024.0,
                kernel: false
            }
        );
        assert!(
            (f.processes[1].resident_mib - 1.2 * 1024.0).abs() < 0.01,
            "a size in g"
        );
    }

    #[test]
    fn an_idle_line_without_its_space() {
        let f = parse(&lines(&FRAME.replace(" 87.5 id", "100.0 id"))).unwrap();
        assert_eq!(f.cpu_percent, 0.0);
    }

    #[test]
    fn actions_and_what_they_refuse() {
        let f = parse(&lines(FRAME)).unwrap();
        let composer = &f.processes[0];
        assert_eq!(Action::End.command(composer), "kill -TERM 812");
        assert_eq!(Action::Lower.command(composer), "renice -n 1 -p 812");
        assert_eq!(
            Action::Raise.command(&Process {
                nice: -20,
                ..composer.clone()
            }),
            "renice -n -20 -p 812"
        );
        assert!(Action::Kill.refused(&f.processes[2]).is_some(), "init");
        assert!(
            Action::End
                .refused(&Process {
                    kernel: true,
                    pid: 9,
                    ..Default::default()
                })
                .is_some()
        );
        assert!(Action::Pause.refused(composer).is_none());
    }

    #[test]
    fn a_power_line_parses_in_the_model_units() {
        let line = "@power DEVTYPE=power_supply POWER_SUPPLY_NAME=test_battery POWER_SUPPLY_STATUS=Discharging POWER_SUPPLY_HEALTH=Good POWER_SUPPLY_TECHNOLOGY=Li-ion POWER_SUPPLY_VOLTAGE_NOW=3300 POWER_SUPPLY_CURRENT_NOW=-1600 POWER_SUPPLY_CAPACITY=50 POWER_SUPPLY_TEMP=26 POWER_SUPPLY_TIME_TO_EMPTY_NOW=3600  usb=1 ac=0";
        let p = parse_power(line).unwrap();
        assert_eq!(
            (
                p.capacity_percent,
                p.voltage_v,
                p.current_a,
                p.temperature_c
            ),
            (50.0, 3.3, -1.6, 26.0)
        );
        assert_eq!(
            (
                p.status.as_str(),
                p.usb_online,
                p.ac_online,
                p.time_to_empty_s
            ),
            ("Discharging", true, false, Some(3600))
        );
        assert!((p.watts() + 5.28).abs() < 1e-4);
        assert_eq!(parse_power("@power usb=1"), None, "no battery");
    }

    #[test]
    fn a_partial_frame_is_none() {
        assert_eq!(
            parse(&lines(
                "top - 07:49:54 up 2:02,  load average: 0.01, 0.20, 1.50"
            )),
            None
        );
    }
}
