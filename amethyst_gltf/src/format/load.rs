use std::collections::HashMap;

/// Trait for loading scene extras.
pub trait Load {
    /// Load extras.
    fn load_index(&mut self, node_map: &HashMap<usize, usize>);
}