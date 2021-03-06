mod long;
mod short;
mod position;

pub use self::long::Long;
pub use self::short::Short;
pub use self::position::Position;

pub struct Key<'help> {
    short: Option<Short>,
    long: Option<Long<'help>>,
    index: Option<Position>
}

impl<'help> Key<'help> {
    pub fn new() -> Self {
        Key { short: None, long: None, index: None }
    }

    pub fn is_positional(&self) -> bool {
        self.index.is_some() || !self.has_switch()
    }

    pub fn has_switch(&self) -> bool {
        self.short.is_some() || self.long.is_some()
    }

    pub fn short(&mut self, short: char) {
        self.short.replace(Short(short));
    }

    pub fn long(&mut self, l: &'help str) {
         self.long.add_long(l);
    }
    pub fn index(&mut self, i: u64) {
        self.index = Some(Position::at(i));
    }

    pub fn hidden_long(&mut self, l: &'help str) {
        self.long.add_hidden_long(l);
    }
}