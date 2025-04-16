use std::sync::OnceLock;
use tokio::runtime::Runtime;

pub static RT: OnceLock<Runtime> = OnceLock::new();
