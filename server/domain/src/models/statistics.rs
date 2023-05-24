use std::fmt::Debug;

#[derive(Debug, Clone)]
pub struct BreakCount(pub u64);

#[derive(Debug, Clone)]
pub struct BuildCount(pub u64);

#[derive(Debug, Clone)]
pub struct PlayTicks(pub u64);

#[derive(Debug, Clone)]
pub struct VoteCount(pub u64);
