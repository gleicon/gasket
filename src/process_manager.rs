use log::info;
use std::convert::TryInto;
use std::env;
use std::process::Command;
use std::sync::Arc;
use std::thread;
use std::time;
use tokio::signal::unix::{signal, Signal, SignalKind};
use tokio::time::{sleep, Duration};

use std::sync::Mutex;

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
        //-> Result<tokio::task::JoinHandle<()>, String> {
        let cmd = self.cmd.clone();
        if self.cmd != "" {
            println!("Spawning: {}", cmd);
            let ms = self.max_spawns.clone();
            let port = self.port.clone();
            //            let _task = actix_web::rt::task::spawn_blocking(move || {
            let _task = std::thread::spawn(move || {
                let arr_cmd: Vec<&str> = cmd.split_whitespace().collect();
                let tx = self.pid_sender;

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

                    tx.lock()
                        .unwrap()
                        .blocking_send((child.id()).try_into().unwrap())
                        .unwrap(); // TODO: capture the error

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
            //  return Ok(task);
        };
        //   Err("Error spawning process: empty command".to_string())
    }

    pub async fn run(cmd: String) {
        let port = env::var("PORT")
            .map(|s| s.parse().unwrap_or(3000))
            .unwrap_or(3000);

        let (tx, rx) = tokio::sync::mpsc::channel(10);

        let mut s = Self {
            self_pid: std::process::id(),
            pid_receiver: Arc::new(Mutex::new(rx)),
            pid_sender: Arc::new(Mutex::new(tx)),
            port: port + 1, // increment port by 1
            max_spawns: MAX_SPAWNS,
            cmd: cmd,
        };

        println!("PORT: {}, child PORT: {}", s.port, s.port + 1);
        let sigchild_stream = signal(SignalKind::child()).unwrap();
        let sigint_stream = signal(SignalKind::interrupt()).unwrap();
        //let sigterm_stream = signal(SignalKind::terminate()).unwrap();

        s.grim_reaper(sigchild_stream).await;
        s.signals_handler(sigint_stream, "SIGINT".to_string()).await;
        // s.signals_handler(sigterm_stream, "SIGTERM".to_string())
        //     .await;

        s.spawn_process(); // blocking process manager
    }

    async fn grim_reaper(
        &mut self,
        mut sigchild_stream: tokio::signal::unix::Signal,
    ) -> tokio::task::JoinHandle<()> {
        println!("Grim reaper activated");

        let rx = Arc::clone(&self.pid_receiver);
        //let mut sigchild_stream = signal(SignalKind::child()).unwrap();

        let signal_task = actix_web::rt::spawn(async move {
            let mut lrx = rx.lock().unwrap();

            loop {
                println!("Looping - chld");
                sigchild_stream.recv().await;
                println!("Looping - chld");

                println!("sigchild received for pid: {:?}", lrx.recv().await);
            }
        });
        signal_task
    }

    async fn signals_handler(
        &mut self,
        mut signal_stream: tokio::signal::unix::Signal,
        signame: String,
    ) -> tokio::task::JoinHandle<()> {
        println!("SIGHANDLER for {} activated", signame);

        let rx = Arc::clone(&self.pid_receiver);

        let signal_task = actix_web::rt::spawn(async move {
            let mut lrx = rx.lock().unwrap();

            loop {
                println!("Looping - {}", signame);
                sleep(Duration::from_millis(1000)).await;
                signal_stream.recv().await;
                println!(
                    "signal {} received, forwarding for pid: {:?}",
                    signame,
                    lrx.recv().await
                );
                std::process::exit(-1);
            }
        });
        signal_task
    }
}
