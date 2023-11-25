pub struct User {
    name: String,
    id: String,
    servers: Option<Vec<String>>,
}

impl User {
    pub fn new(name: String, id: String, servers: Option<Vec<String>>) -> Self {
        Self { name, id, servers }
    }

}
