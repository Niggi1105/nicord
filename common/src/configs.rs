pub struct ServerConfig {
    priviledges: Vec<Priviledge>,
}

pub struct Priviledge {
    name: String,
    level: u8,
}
