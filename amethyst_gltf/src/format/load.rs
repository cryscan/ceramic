use std::collections::HashMap;

/// Trait for loading scene extras.
pub trait Load {
    /// Load extras.
    fn load(&mut self, node_map: &HashMap<usize, usize>);
}