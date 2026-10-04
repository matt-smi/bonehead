/// Logical Discord channels to route output to
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Target {
    Default,
    Hockey,
}

#[derive(Clone, Debug)]
pub enum Outbound {
    Message { target: Target, text: String },
}
