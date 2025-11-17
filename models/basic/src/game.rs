pub trait Game {
    fn handle_message(&self);
    fn dispatch_message(&self);
}

pub enum ServerMessageError {
    Rejected(MessageRejection),
}

pub struct MessageRejection {
    pub reason: String,
}