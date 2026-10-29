mod backend;
mod controls;
mod input;
mod key_names;
mod layers;
mod screenshot;
mod splash;
mod state;
mod status;
mod touches;

use std::{path::PathBuf, process::Command, time::Instant};

use blackbird_device::Description;
use calloop::signals::{Signal, Signals};
use smithay::reexports::{calloop::EventLoop, wayland_server::Display};
use tracing::{error, info, warn};
use tracing_subscriber::{EnvFilter, prelude::*};

use state::Composer;

/// The developer activities the composer can start in place of the product (SYS-071), by name, until
/// the orchestrator's activity launcher exists.
const ACTIVITIES: &[(&str, &str)] = &[("ui-primitives", "/usr/lib/blackbird/testbed")];

/// The activity asked for by the flag: `BLACKBIRD_ACTIVITY`, or the name in `/run/blackbird/composer/activity`.
fn activity() -> Option<&'static str> {
    let name = std::env::var("BLACKBIRD_ACTIVITY")
        .ok()
        .or_else(|| std::fs::read_to_string(blackbird_platform::run::ACTIVITY).ok())?;
    let name = name.trim();
    match ACTIVITIES.iter().find(|(n, _)| *n == name) {
        Some((_, program)) => Some(program),
        None => {
            let known: Vec<&str> = ACTIVITIES.iter().map(|(n, _)| *n).collect();
            error!(name, ?known, "no such activity: starting the product");
            None
        }
    }
}

struct Args {
    description: PathBuf,
    /// The product host, started once the display is up, until the orchestrator starts it.
    client: Vec<String>,
}

fn args() -> Args {
    let mut description = blackbird_device::locate::find()
        .unwrap_or_else(|| PathBuf::from(blackbird_device::locate::SEARCH_PATHS[0]));
    let mut client = Vec::new();
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--description" => description = it.next().map(PathBuf::from).unwrap_or(description),
            "--" => {
                client = it.collect();
                break;
            }
            other => {
                eprintln!(
                    "composer: unknown argument {other}\nusage: composer [--description FILE] [-- COMMAND...]"
                );
                std::process::exit(2);
            }
        }
    }
    Args {
        description,
        client,
    }
}

fn logging() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let registry = tracing_subscriber::registry().with(filter);
    // Native journal fields on the device (ADR 0008); plain stderr elsewhere.
    match tracing_journald::layer() {
        Ok(journald) if std::env::var_os("JOURNAL_STREAM").is_some() => {
            registry.with(journald).init()
        }
        _ => registry
            .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
            .init(),
    }
}

pub fn main() {
    logging();
    // The boot modes the power menu offers, fetched now so opening the menu never waits (T1-047).
    crate::power::client().prefetch_modes();
    let args = args();

    let description = match std::fs::read_to_string(&args.description)
        .map_err(|e| e.to_string())
        .and_then(|text| Description::parse(&text).map_err(|e| e.to_string()))
    {
        Ok(d) => d,
        Err(err) => {
            error!(path = %args.description.display(), %err, "device description");
            std::process::exit(1);
        }
    };
    info!(device = %description.identity.id, "starting");

    let mut event_loop: EventLoop<'static, Composer> = EventLoop::try_new().expect("event loop");
    let display: Display<Composer> = Display::new().expect("Wayland display");
    let mut composer = Composer::new(event_loop.handle(), display, description);

    if let Err(err) = backend::init(&mut composer) {
        error!(%err, "display");
        std::process::exit(1);
    }
    info!(socket = ?composer.socket_name, "listening");

    // Asked to stop (systemd's SIGTERM): hand the display to the boot UI, then exit. Set up before
    // any thread starts, so every thread keeps these signals blocked for the event loop; the
    // product host, a child process, starts with them unblocked.
    let signals = Signals::new(&[Signal::SIGTERM, Signal::SIGINT]).expect("signals");
    event_loop
        .handle()
        .insert_source(signals, |_, _, composer| {
            if let Some(backend) = composer.backend.as_ref() {
                backend.hand_over();
            }
            info!("stopped");
            std::process::exit(0);
        })
        .expect("cannot watch for signals");

    // Until the orchestrator owns activities (ADR 0002), the composer supervises the product host: it
    // restarts it when it exits, and the display keeps the last frame meanwhile.
    // The activity flag, when set, starts that activity in place of the product.
    let client = match activity() {
        Some(program) => {
            info!(
                program,
                "activity flag: starting it in place of the product"
            );
            vec![program.to_string()]
        }
        None => args.client,
    };
    if !client.is_empty() {
        supervise_product(client, composer.socket_name.clone());
    }

    event_loop
        .run(None, &mut composer, |composer| {
            let _ = composer.display_handle.flush_clients();
        })
        .expect("event loop");
    std::process::exit(1);
}

/// Runs the product host for as long as the composer runs, restarting it when it exits, after
/// crate::restart's backoff.
fn supervise_product(command: Vec<String>, socket: std::ffi::OsString) {
    std::thread::Builder::new()
        .name("product-host".into())
        .spawn(move || {
            let (program, rest) = command.split_first().expect("a command");
            let mut backoff = crate::restart::Backoff::default();
            loop {
                let started = Instant::now();
                let outcome = Command::new(program).args(rest).env("WAYLAND_DISPLAY", &socket).spawn().and_then(|mut child| {
                    info!(pid = child.id(), %program, "product host started");
                    child.wait()
                });
                let wait = backoff.after_exit(started.elapsed());
                match outcome {
                    Ok(status) => warn!(%status, restart_in_ms = wait.as_millis() as u64, "product host exited"),
                    Err(err) => error!(%err, %program, restart_in_ms = wait.as_millis() as u64, "product host"),
                }
                std::thread::sleep(wait);
            }
        })
        .expect("cannot start the product host's supervisor");
}
