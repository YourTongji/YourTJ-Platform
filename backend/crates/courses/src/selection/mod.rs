//! Selection (选课/PK) domain — a read-only mirror of 一系统 synced into the
//! `selection` PostgreSQL schema. All tables live under `selection.*` but the
//! module lives inside the courses crate because selection is a sub-domain of
//! the course catalogue.

pub mod dto;
pub mod models;
