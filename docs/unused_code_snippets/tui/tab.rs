#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Chat,
    Peers,
    Direct,
    Log,
}

impl Tab {
    pub fn index(&self) -> usize {
        match self {
            Tab::Chat => 0,
            Tab::Peers => 1,
            Tab::Direct => 2,
            Tab::Log => 3,
        }
    }

    pub fn from_index(idx: usize) -> Self {
        match idx {
            0 => Tab::Chat,
            1 => Tab::Peers,
            2 => Tab::Direct,
            3 => Tab::Log,
            _ => Tab::Chat,
        }
    }

    pub fn total_tabs(&self) -> usize {
        4
    }
}

/// Represents content that can be displayed in a tab
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TabContent {
    Chat,
    Peers,
    Direct(String),
    Log,
}

impl TabContent {
    pub fn peer_id(&self) -> Option<&str> {
        match self {
            TabContent::Direct(id) => Some(id),
            _ => None,
        }
    }

    pub fn is_input_enabled(&self) -> bool {
        matches!(self, TabContent::Chat | TabContent::Direct(_))
    }
}
