pub struct Parser {
    buf: Vec<u8>,
}

impl Parser {
    pub fn new() -> Parser {
        Parser {
            buf: Vec![0; 4096]
        }
    }
    pub fn recv_from
}