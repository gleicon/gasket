use log::info;
use std::convert::TryInto;
use std::env;
use std::process::Command;
use std::sync::Arc;
use std::thread;
use std::time;
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
//use tokio::sync::Mutex;
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
            info!("Spawning: {}", cmd);
            println!("Spawning: {}", cmd);
            let ms = self.max_spawns.clone();
            let port = self.port.clone();
            // // #![feature(async_closure)]
            let task = actix_web::rt::task::spawn_blocking(move || {
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
        // println!("Installing signal handlers");

        // let signals = Signals::new(&[SIGHUP, SIGTERM, SIGINT, SIGQUIT, SIGCHLD]).unwrap();
        // let handle = signals.handle();
        s.grim_reaper().await;
        s.spawn_process();
        // match s.spawn_process().await {
        //     Ok(task) => task.await.unwrap(),
        //     Err(e) => println!("{}", e),
        // }
    }

    async fn grim_reaper(&mut self) -> tokio::task::JoinHandle<()> {
        println!("Grim reaper activated");

        let rx = Arc::clone(&self.pid_receiver);
        let signal_task = actix_web::rt::spawn(async move {
            let mut stream = signal(SignalKind::child()).unwrap(); //|| SignalKind::child())?;
            let mut lrx = rx.lock().unwrap();

            loop {
                println!("Looping");
                let evt = stream.recv().await;
                println!("signal received: {:?} for pid: {:?}", evt, lrx.recv().await);
            }
        });
        signal_task
    }
}
