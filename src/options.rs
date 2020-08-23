use std::{fmt, result, str::FromStr};

pub fn build_path(controller_opts: &ControllerOptions, tag_opts: &TagOptions) -> String {
    let buf = &mut vec![];
    controller_opts.append_to(buf);
    tag_opts.append_to(buf);
    buf.join("&")
}

pub fn generate_id(controller_opts: &ControllerOptions, tag_opts: &TagOptions) -> String {
    let buf = &mut vec![];
    controller_opts.append_to(buf);
    if let Some(ref name) = tag_opts.name {
        buf.push(format!("name={}", name));
    }
    buf.join("&")
}

/// controller options
#[derive(Debug)]
pub enum ControllerOptions {
    EIP {
        plc: Option<PlcKind>,
        gateway: Option<String>,
        path: Option<String>,
        use_connected_msg: Option<bool>,
    },
    ModBus {
        gateway: Option<String>,
        path: Option<String>,
    },
}

impl Clone for ControllerOptions {
    fn clone(&self) -> Self {
        match self {
            Self::EIP {
                ref plc,
                ref gateway,
                ref path,
                ref use_connected_msg,
            } => Self::EIP {
                plc: plc.map(|x| x.clone()),
                gateway: gateway.as_ref().map(|x| x.clone()),
                path: path.as_ref().map(|x| x.clone()),
                use_connected_msg: use_connected_msg.as_ref().map(|x| x.clone()),
            },
            Self::ModBus {
                ref gateway,
                ref path,
            } => Self::ModBus {
                gateway: gateway.as_ref().map(|x| x.clone()),
                path: path.as_ref().map(|x| x.clone()),
            },
        }
    }
}

// impl Copy for ControllerOptions {}

impl ControllerOptions {
    /// validate controller options
    pub fn validate(&self) -> result::Result<(), String> {
        match self {
            Self::EIP {
                ref plc,
                ref gateway,
                ref path,
                ..
            } => {
                if plc.is_none() {
                    return Err(String::from("plc required"));
                }
                if gateway.is_none() {
                    return Err(String::from("gateway required"));
                }
                let plc = plc.unwrap();
                if plc == PlcKind::ControlLogix && path.is_none() {
                    return Err(String::from("path required for controllogix"));
                }
                if plc == PlcKind::Micro800 && path.is_some() {
                    return Err(String::from("path must not specified for micro800"));
                }
                Ok(())
            }
            Self::ModBus {
                ref gateway,
                ref path,
            } => {
                if gateway.is_none() {
                    return Err(String::from("gateway required"));
                }
                if path.is_none() {
                    return Err(String::from("path required"));
                }
                Ok(())
            }
        }
    }

    fn append_to(&self, buf: &mut Vec<String>) {
        match self {
            ControllerOptions::EIP {
                ref plc,
                ref gateway,
                ref path,
                ref use_connected_msg,
            } => {
                buf.push("protocol=ab-cip".to_owned());
                if let Some(plc) = plc {
                    buf.push(format!("plc={}", plc));
                }
                if let Some(gateway) = gateway {
                    buf.push(format!("gateway={}", gateway))
                }
                if let Some(path) = path {
                    buf.push(format!("path={}", path));
                }
                if let Some(use_connected_msg) = use_connected_msg {
                    buf.push(format!("path={}", *use_connected_msg as u8));
                }
            }
            ControllerOptions::ModBus {
                ref gateway,
                ref path,
            } => {
                if let Some(gateway) = gateway {
                    buf.push(format!("gateway={}", gateway).to_owned())
                }
                if let Some(path) = path {
                    buf.push(format!("path={}", path).to_owned());
                }
            }
        }
    }
}

#[derive(Default, Debug)]
pub struct TagOptions {
    elem_count: Option<u32>,
    elem_size: Option<u32>,
    read_cache_ms: Option<u32>,
    name: Option<String>,
}

impl FromStr for TagOptions {
    type Err = String;
    fn from_str(s: &str) -> result::Result<TagOptions, Self::Err> {
        let mut opts: TagOptions = Default::default();
        let items = s.split("&").map(|s| {
            let part: Vec<&str> = s.split("=").collect();
            (part[0], part[1])
        });
        for item in items {
            if item.0 == "elem_count" {
                opts.elem_count = Some(
                    u32::from_str(item.1)
                        .map_err(|_| String::from("failed to parse elem_count"))?,
                );
            }
            if item.0 == "elem_size" {
                opts.elem_size = Some(
                    u32::from_str(item.1).map_err(|_| String::from("failed to parse elem_size"))?,
                );
            }

            if item.0 == "read_cache_ms" {
                opts.read_cache_ms = Some(
                    u32::from_str(item.1)
                        .map_err(|_| String::from("failed to parse read_cache_ms"))?,
                );
            }

            if item.0 == "name" {
                opts.name = Some(String::from(item.1));
            }
        }

        Ok(opts)
    }
}

impl TagOptions {
    fn append_to(&self, buf: &mut Vec<String>) {
        if let Some(ref name) = self.name {
            buf.push(format!("name={}", name));
        }
        if let Some(ref elem_size) = self.elem_size {
            buf.push(format!("elem_size={}", elem_size));
        }
        if let Some(ref elem_count) = self.elem_count {
            buf.push(format!("elem_size={}", elem_count));
        }
        if let Some(ref read_cache_ms) = self.read_cache_ms {
            buf.push(format!("elem_size={}", read_cache_ms));
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Protocol {
    EIP,
    ModBus,
}

impl fmt::Display for Protocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Protocol::EIP => write!(f, "ab-eip"),
            Protocol::ModBus => write!(f, "modbus-tcp"),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum PlcKind {
    /// Tell the library that this tag is in a Control Logix-class PLC
    ControlLogix,
    /// Tell the library that this tag is in a PLC/5 PLC
    PLC5,
    /// Tell the library that this tag is in a SLC 500 PLC
    SLC500,
    /// Tell the library that this tag is in a Control Logix-class PLC using the PLC/5 protocol
    LogixPCCC,
    /// Tell the library that this tag is in a Micro800-class PLC
    Micro800,
    /// Tell the library that this tag is in a Micrologix PLC
    MicroLogix,
}

impl fmt::Display for PlcKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlcKind::ControlLogix => write!(f, "controllogix"),
            PlcKind::PLC5 => write!(f, "plc5"),
            PlcKind::SLC500 => write!(f, "slc500"),
            PlcKind::LogixPCCC => write!(f, "logixpccc"),
            PlcKind::Micro800 => write!(f, "micro800"),
            PlcKind::MicroLogix => write!(f, "micrologix"),
        }
    }
}
