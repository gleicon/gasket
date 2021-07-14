use log::info;
use std::env;
use std::process::Command;
use std::thread;
use std::time;

pub struct ProcessManager {
    pub self_pid: u32,
    pub port: u32,
    pub max_spawns: u32,
}
const MAX_SPAWNS: u32 = 5;

impl ProcessManager {
    pub fn spawn_process(&mut self, cmd: String) {
        if cmd != "" {
            info!("Spawning: {}", cmd);
            let ms = self.max_spawns;

            thread::spawn(move || {
                let arr_cmd: Vec<&str> = cmd.split_whitespace().collect();
                println!("{:?}", arr_cmd);
                let cmd = arr_cmd[0].clone();

                let cleanup_time = time::Duration::from_secs(1);
                let mut respawn_counter = 0;
                loop {
                    let st = Command::new(cmd)
                        .args(&arr_cmd[1..arr_cmd.len()])
                        .status()
                        .expect("sh command failed to start");
                    match st.code() {
                        Some(code) => println!("Process exited with status code: {}", code),
                        None => println!("Process terminated by signal"),
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

    pub fn new() -> Self {
        let port = env::var("PORT")
            .map(|s| s.parse().unwrap_or(3000))
            .unwrap_or(3000);

        let s = Self {
            self_pid: std::process::id(),
            port: port,
            max_spawns: MAX_SPAWNS,
        };
        if s.self_pid == 1 {
            info!("Running as PID1");
            // set signal handlers and grimreaper for zombies
        }
        info!("PORT: {}, child PORT: {}", s.port, s.port + 1);
        return s;
    }
}
