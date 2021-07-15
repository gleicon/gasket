use libc;
use log::info;
use std::convert::TryInto;
use std::env;
use std::process::Command;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time;

pub struct ProcessManager {
    pub self_pid: u32,
    pub child_process: i32,
    pub port: u32,
    pub max_spawns: u32,
}

pub struct StaticProcessManager {
    pm: Arc<Mutex<ProcessManager>>,
}
const MAX_SPAWNS: u32 = 5;

impl StaticProcessManager {
    pub async fn spawn_process(&mut self, cmd: String) {
        if cmd != "" {
            info!("Spawning: {}", cmd);
            let local_process_manager = self.pm.clone();

            thread::spawn(move || {
                let ms = local_process_manager.lock().unwrap().max_spawns;
                let port = local_process_manager.lock().unwrap().port;

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
                    local_process_manager.lock().unwrap().child_process =
                        (child.id()).try_into().unwrap();

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
        let s = Self {
            pm: Arc::new(Mutex::new(ProcessManager::new().await)),
        };
        return s;
    }
}

impl ProcessManager {
    pub async fn new() -> Self {
        let port = env::var("PORT")
            .map(|s| s.parse().unwrap_or(3000))
            .unwrap_or(3000);

        let mut s = Self {
            self_pid: std::process::id(),
            port: port + 1, // increment port by 1
            max_spawns: MAX_SPAWNS,
            child_process: 0,
        };

        if s.self_pid == 1 {
            info!("Running as PID1");
            // set signal handlers and grimreaper for zombies

            // Launch the child process requested.
            // Install a SIGCHLD signal handler, which will indicate that a child or orphan process is ready to be reaped.
            // Install a SIGINT signal handler which will send a SIGINT to the child process. This will make Ctrl-C work.
            // Start a loop that reaps a child each time SIGCHLD occurs.
            // let term = Arc::new(AtomicBool::new(false));
            // let child = Arc::new(AtomicBool::new(false));
            // let _ = signal_hook::flag::register(libc::SIGINT, Arc::clone(&term));
            // let _ = signal_hook::flag::register(libc::SIGCHLD, Arc::clone(&child));
        }
        info!("PORT: {}, child PORT: {}", s.port, s.port + 1);
        s.grim_reaper().await.unwrap();
        return s;
    }

    async fn grim_reaper(&mut self) -> Result<(), std::io::Error> {
        let till = self.child_process.clone();
        if till == 0 {
            ()
        }
        let mut status = 0;
        let mut stream = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::child())?;
        tokio::spawn(async move {
            while stream.recv().await.is_some() {
                loop {
                    let pid = unsafe { libc::waitpid(-1, &mut status, libc::WNOHANG) };
                    if pid == till {
                        return ();
                    } else if pid <= 0 {
                        break;
                    }
                }
            }
            //return Ok(());
        });
        // while stream.recv().await.is_some() {
        //     loop {
        //         let pid = unsafe { libc::waitpid(-1, &mut status, libc::WNOHANG) };
        //         if pid == till {
        //             return Ok(());
        //         } else if pid <= 0 {
        //             break;
        //         }
        //     }
        // }
        Ok(())
    }
}
