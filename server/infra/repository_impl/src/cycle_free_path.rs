use std::collections::HashSet;
use std::error::Error;
use std::fmt::{Debug, Display};
use std::hash::Hash;

#[derive(Debug)]
pub struct CycleFoundError<N> {
    pub cycle: Vec<N>,
}

impl<N: Debug> Display for CycleFoundError<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} contains a cycle", self.cycle)
    }
}

impl<N: Debug> Error for CycleFoundError<N> {}

pub fn construct_cycle_free_path<N>(
    initial_node: N,
    mut next_node: impl FnMut(N) -> Option<N>,
) -> Result<Vec<N>, CycleFoundError<N>>
where
    N: Eq + Hash + Clone,
{
    let mut path = vec![initial_node.clone()];

    let mut visited = HashSet::new();
    visited.insert(initial_node.clone());

    let mut current_node = initial_node;
    while let Some(next_node) = next_node(current_node) {
        if visited.contains(&next_node) {
            path.push(next_node);
            return Err(CycleFoundError { cycle: path });
        }

        visited.insert(next_node.clone());
        path.push(next_node.clone());
        current_node = next_node;
    }

    Ok(path)
}
