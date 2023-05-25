use std::fmt::Debug;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BreakCount(pub u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildCount(pub u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayTicks(pub u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoteCount(pub u64);
