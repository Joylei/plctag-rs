use plctag::future::prelude::*;
use std::sync::Arc;
use tokio::task;
use tokio::{self, runtime::Runtime};

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
    fn run(&self, ctx: Processor) -> tokio::task::JoinHandle<Result<()>> {
        task::spawn(async move {
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
            let size = tag1.size().await?;
            let buf = Buf::new(size as usize);
            //let buf = unsafe { Pin::new_unchecked(&mut buf) };
            let (buf, _) = tag1.get_bytes(buf).await?;
            tag2.set_bytes(buf).await?;
            Ok(())
        })
    }

    fn expired(&self) -> bool {
        false
    }
}

fn main() {
    let config = ControllerOptions::new(PLC_HOST);
    let controller = Arc::new(Controller::from(config));
    let controller1 = Arc::clone(&controller);

    let mut rt = Runtime::new().unwrap();
    rt.block_on(async move {
        let _task = tokio::spawn(async move {
            controller.scan().await
        });

        //add tags
        let tag1 = MyTagBuilder::new("MyTag1", "protocol=ab-eip&plc=controllogix&path=1,0&gateway=192.168.1.120&name=MyTag1&elem_count=1&elem_size=16");
        let tag2 = MyTagBuilder::new("MyTag2", "protocol=ab-eip&plc=controllogix&path=1,0&gateway=192.168.1.120&name=MyTag2&elem_count=1&elem_size=16");

        let res1 = controller1.ensure_tag(tag1.clone()).await;
        assert!(res1.is_some());
        let res2 = controller1.ensure_tag(tag2.clone()).await;
        assert!(res2.is_some());

        //post operations to controller
        for _ in 0..1000 {
            controller1.post(PingPong {}).await;
        }

        drop(_task);
    });
}
