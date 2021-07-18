use libc;
use log::info;
use std::convert::TryInto;
use std::env;
use std::process::Command;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use std::io::Error;

use signal_hook::consts::signal::*;
use signal_hook_tokio::Signals;

use futures::stream::StreamExt;

#[derive(Clone, Copy)]
pub struct ProcessManager {
    pub child_process: i32,
}

pub struct StaticProcessManager {
    pm: Arc<Mutex<ProcessManager>>,
    pid_sender: Arc<Mutex<UnboundedSender<u32>>>,
    pid_receiver: UnboundedReceiver<u32>,
    pub port: u32,
    pub max_spawns: u32,
    pub self_pid: u32,
}
const MAX_SPAWNS: u32 = 5;

impl StaticProcessManager {
    pub async fn spawn_process(&mut self, cmd: String) {
        if cmd != "" {
            info!("Spawning: {}", cmd);
            let local_pid_sender = self.pid_sender.clone();
            let ms = self.max_spawns;
            let port = self.port;

            tokio::spawn(async move {
                let arr_cmd: Vec<&str> = cmd.split_whitespace().collect();
                println!("{:?}", arr_cmd);
                let cmd = arr_cmd[0].clone();

                let cleanup_time = time::Duration::from_secs(1);
                let mut respawn_counter = 0;
                loop {
                    let mut child = Command::new(cmd)
                        .args(&arr_cmd[1..arr_cmd.len()])
                        .env("PORT", port.to_string())
                        .spawn()
                        .unwrap();
                    println!("Spawning new process: {}", child.id());
                    // local_pid_sender
                    //     .lock()
                    //     .unwrap()
                    //     .send((child.id()).try_into().unwrap())
                    //     .unwrap();

                    match child.wait() {
                        Ok(c) => match c.code() {
                            Some(code) => println!("Process exited with status code: {}", code),
                            None => println!("Process terminated by signal"),
                        },
                        Err(e) => println!("{}", e),
                    }

                    respawn_counter = respawn_counter + 1;
                    if respawn_counter > ms {
                        println!("Process spawning too much, aborting gasket");
                        std::process::exit(-1);
                    }
                    println!("sleeping before respawn {}", respawn_counter);
                    thread::sleep(cleanup_time);
                }
            });
        };
    }

    pub async fn new() -> Self {
        let port = env::var("PORT")
            .map(|s| s.parse().unwrap_or(3000))
            .unwrap_or(3000);

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

        let mut s = Self {
            self_pid: std::process::id(),
            pm: Arc::new(Mutex::new(ProcessManager::new().await)),
            pid_receiver: rx,
            pid_sender: Arc::new(Mutex::new(tx)),
            port: port + 1, // increment port by 1
            max_spawns: MAX_SPAWNS,
        };

        println!("PORT: {}, child PORT: {}", s.port, s.port + 1);
        println!("Installing signal handlers");

        let signals = Signals::new(&[SIGHUP, SIGTERM, SIGINT, SIGQUIT, SIGCHLD]).unwrap();
        let handle = signals.handle();
        //let signals_task = tokio::spawn(s.grim_reaper(signals));
        s.grim_reaper(signals).await;
        // s.grim_reaper().await.unwrap();
        return s;
    }
    async fn grim_reaper(&mut self, signals: Signals) {
        let mut signals = signals.fuse();
        let signal_task = tokio::spawn(async move {
            while let Some(signal) = signals.next().await {
                match signal {
                    SIGHUP => {
                        // Reload configuration
                        // Reopen the log file
                    }
                    SIGCHLD => {
                        // reap
                    }
                    SIGTERM | SIGINT | SIGQUIT => {
                        // Shutdown the system;
                    }
                    _ => unreachable!(),
                }
            }
        });
    }
}

impl ProcessManager {
    pub async fn new() -> Self {
        let s = Self { child_process: 0 };

        println!("Installing signal handlers");
        let signals = Signals::new(&[SIGHUP, SIGTERM, SIGINT, SIGQUIT, SIGCHLD]).unwrap();
        let handle = signals.handle();
        let signals_task = tokio::spawn(s.grim_reaper(signals));
        // s.grim_reaper().await.unwrap();
        return s;
    }

    async fn grim_reaper(self: Self, signals: Signals) {
        let mut signals = signals.fuse();
        while let Some(signal) = signals.next().await {
            match signal {
                SIGHUP => {
                    // Reload configuration
                    // Reopen the log file
                }
                SIGCHLD => {
                    // reap
                }
                SIGTERM | SIGINT | SIGQUIT => {
                    // Shutdown the system;
                }
                _ => unreachable!(),
            }
        }
    }
}
