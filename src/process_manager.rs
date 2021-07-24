use log::info;
use std::convert::TryInto;
use std::env;
use std::process::Command;
use std::sync::Arc;
use std::thread;
use std::time;

use futures::stream::StreamExt;
use signal_hook::consts::signal::*;
use signal_hook_tokio::Signals;

use std::sync::Mutex;

#[derive(Clone)]
pub struct StaticProcessManager {
    pid_sender: Arc<Mutex<tokio::sync::mpsc::Sender<u32>>>,
    pid_receiver: Arc<Mutex<tokio::sync::mpsc::Receiver<u32>>>,
    pub port: u32,
    pub max_spawns: u32,
    pub self_pid: u32,
    pub cmd: String,
}
const MAX_SPAWNS: u32 = 5;

impl StaticProcessManager {
    pub fn spawn_process(self: Self) {
        let cmd = self.cmd.clone();

        if self.cmd != "" {
            info!("Spawning: {}", cmd);
            let ms = self.max_spawns.clone();
            let port = self.port.clone();
            let _task = actix_web::rt::task::spawn_blocking(move || {
                //let _task = std::thread::spawn(move || {
                let arr_cmd: Vec<&str> = cmd.split_whitespace().collect();
                let tx = self.pid_sender;

                let cmd = arr_cmd[0].clone();

                let cleanup_time = time::Duration::from_secs(1);
                let mut respawn_counter = 0;
                loop {
                    let mut child = match Command::new(cmd)
                        .args(&arr_cmd[1..arr_cmd.len()])
                        .env("PORT", port.to_string())
                        .spawn()
                    {
                        Ok(child) => child,
                        Err(e) => {
                            info!("Error: {}: {} - exiting", cmd, e);
                            std::process::exit(-1);
                        }
                    };
                    info!("Spawned process pid: {}", child.id());

                    tx.lock()
                        .unwrap()
                        .blocking_send((child.id()).try_into().unwrap())
                        .unwrap(); // TODO: capture the error

                    match child.wait() {
                        Ok(c) => match c.code() {
                            Some(code) => info!("Process exited with status code: {}", code),
                            None => info!("Process terminated by signal"),
                        },
                        Err(e) => info!("{}", e),
                    }

                    respawn_counter = respawn_counter + 1;
                    if respawn_counter > ms {
                        info!("Process spawning too much, aborting gasket");
                        std::process::exit(-1);
                    }
                    info!("Sleeping before respawn {}", respawn_counter);
                    thread::sleep(cleanup_time);
                }
            });
        };
    }

    pub async fn run(cmd: String) -> signal_hook_tokio::Handle {
        let port = env::var("PORT")
            .map(|s| s.parse().unwrap_or(3000))
            .unwrap_or(3000);

        let (tx, rx) = tokio::sync::mpsc::channel(10);

        let s = Self {
            self_pid: std::process::id(),
            pid_receiver: Arc::new(Mutex::new(rx)),
            pid_sender: Arc::new(Mutex::new(tx)),
            port: port + 1, // increment port by 1
            max_spawns: MAX_SPAWNS,
            cmd: cmd,
        };

        info!("Env vars: PORT: {}, child PORT: {}", s.port, s.port + 1);
        let signals = Signals::new(&[SIGHUP, SIGTERM, SIGINT, SIGQUIT, SIGCHLD]).unwrap();

        let handle = signals.handle();

        s.clone().signals_handler(signals).await;

        s.clone().spawn_process(); // blocking process manager
        return handle;
    }

    async fn grim_reaper(&mut self, pid_t: i32) -> tokio::task::JoinHandle<()> {
        let signal_task = actix_web::rt::spawn(async move {
            let mut st = 0;
            loop {
                let lpid = unsafe { libc::waitpid(-1, &mut st, libc::WNOHANG) };
                info!("Capturing zombie {}", lpid);
                if lpid == pid_t {
                    return ();
                } else if lpid <= 0 {
                    break;
                }
            }
            ()
        });
        signal_task
    }

    async fn signals_handler(mut self, signals: Signals) -> tokio::task::JoinHandle<()> {
        let rx = Arc::clone(&self.pid_receiver);
        let signal_task = actix_web::rt::spawn(async move {
            let mut signals = signals.fuse();
            while let Some(signal) = signals.next().await {
                let pid = rx.lock().unwrap().recv().await.unwrap();
                let pid_t: libc::pid_t = match pid.try_into() {
                    Ok(x) => x,
                    Err(_e) => -1,
                };
                self.grim_reaper(pid_t).await;
                //GrimReaper::new().unwrap().reap(pid_t).await.unwrap();
                match signal {
                    SIGCHLD => {
                        info!("SIGCHLD captured");
                    }
                    SIGHUP => {
                        info!("SIGHUP");
                    }

                    SIGINT => unsafe {
                        libc::kill(pid_t, libc::SIGINT);
                    },

                    SIGTERM | SIGQUIT => unsafe {
                        libc::kill(pid_t, libc::SIGTERM);
                    },
                    _ => unreachable!(),
                }
            }
        });
        signal_task
    }
}
