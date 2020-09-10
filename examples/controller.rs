use plctag::{controller::*, Result};
use std::sync::Arc;
use std::thread;

static PLC_HOST: &str = "192.168.1.120";

#[derive(Clone)]
struct MyTagBuilder {
    name: String,
    path: String,
}

impl MyTagBuilder {
    pub fn new(name: &str, path: &str) -> Self {
        Self {
            name: name.to_owned(),
            path: path.to_owned(),
        }
    }
}

impl TagOptions for MyTagBuilder {
    fn host(&self) -> String {
        PLC_HOST.to_owned()
    }

    fn name(&self) -> String {
        self.name.clone()
    }

    fn path(&self) -> String {
        self.path.clone()
    }
}

struct PingPong {}

impl Operation for PingPong {
    fn id(&self) -> usize {
        1
    }

    /// get & set value here
    fn run<'a>(&self, ctx: &'a dyn Context) -> Result<()> {
        let res = ctx.find_tag("MyTag1");
        let tag1 = match res {
            Some(tag) => tag,
            _ => return Ok(()),
        };
        let res = ctx.find_tag("MyTag2");
        let tag2 = match res {
            Some(tag) => tag,
            _ => return Ok(()),
        };

        //read write whatever type
        let size = tag1.size()?;
        let mut buf: Vec<u8> = Vec::with_capacity(size as usize);
        tag1.get_bytes(&mut buf[..])?;
        tag2.set_bytes(&buf)?;
        Ok(())
    }

    fn expired(&self) -> bool {
        false
    }
}

fn main() {
    let config = ControllerOptions::default().host(PLC_HOST);
    let controller = Arc::new(Controller::from(config));
    let controller1 = Arc::clone(&controller);

    //run controller in another thread
    let thread1 = thread::spawn(move || controller.scan());

    //add tags
    let tag1 = MyTagBuilder::new("MyTag1", "protocol=ab-eip&plc=controllogix&path=1,0&gateway=192.168.1.120&name=MyTag1&elem_count=1&elem_size=16");
    let tag2 = MyTagBuilder::new("MyTag2", "protocol=ab-eip&plc=controllogix&path=1,0&gateway=192.168.1.120&name=MyTag2&elem_count=1&elem_size=16");

    let res1 = controller1.ensure_tag(tag1.clone());
    assert!(res1.is_some());
    let res2 = controller1.ensure_tag(tag2.clone());
    assert!(res2.is_some());

    //post operations to controller
    for _ in 0..1000 {
        controller1.post(PingPong {});
    }

    thread1.join().unwrap();
}
