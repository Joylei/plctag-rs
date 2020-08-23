extern crate plctag;

use std::env;
use std::process::{Child, Command};
use std::sync::{Arc, Mutex};

// static server: Arc<Mutex<Option<Child>>> = empty();

// fn empty() -> Arc<Mutex<Option<Child>>> {
//     Arc::new(Mutex::new(None))
// }

// fn spawn_server() -> Arc<Mutex<Option<Child>>> {
//     let s = &mut *server.lock().unwrap();
//     if s.is_none() {
//         let p = Command::new("ab_server.exe")
//             .arg("--plc=ControlLogix")
//             .arg("--path=1,0")
//             .arg("--tag=TagInt:SINT[1]")
//             .spawn()
//             .unwrap();
//         *s = Some(p);
//     }
//     Arc::clone(&server)
// }
