use std::collections::HashSet;
use std::fmt::Debug;
use std::hash::Hash;
use thiserror::Error;

#[derive(Debug, Error)]
#[error("{cycle:?} contains a cycle")]
pub struct CycleFoundError<N> {
    pub cycle: Vec<N>,
}

pub fn construct_cycle_free_path<N>(
    initial_node: N,
    next_node: impl Fn(N) -> Option<N>,
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
