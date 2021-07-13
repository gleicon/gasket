use log::info;
use std::process::Command;
use std::thread;
use std::time;

pub struct ProcessManager {
    pub self_pid: u32,
}

impl ProcessManager {
    pub fn spawn_process(&mut self, cmd: &str) {
        if cmd != "" {
            info!("Spawning: {}", cmd);

            thread::spawn(move || {
                let cleanup_time = time::Duration::from_secs(1);
                let mut respawn_counter = 0;
                loop {
                    let st = Command::new("python")
                        .args(["-mSimpleHTTPServer", "3000"])
                        .status()
                        .expect("sh command failed to start");
                    match st.code() {
                        Some(code) => println!("Process exited with status code: {}", code),
                        None => println!("Process terminated by signal"),
                    }

                    respawn_counter = respawn_counter + 1;
                    if respawn_counter > 1 {
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
        let s = Self {
            self_pid: std::process::id(),
        };
        return s;
    }
}
