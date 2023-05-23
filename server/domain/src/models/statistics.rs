use std::fmt::Debug;

#[derive(Debug, Clone)]
pub struct BreakCount(u64);

#[derive(Debug, Clone)]
pub struct BuildCount(u64);

#[derive(Debug, Clone)]
pub struct PlayTicks(u64);

#[derive(Debug, Clone)]
pub struct VoteCount(u64);
