#![allow(unused)]

use hvp_archive::archive::rebuild_progress::RebuildProgress;

pub const OBSCURE1_HVP: &str = "tests/resources/obscure1.hvp";
pub const OBSCURE2_HVP: &str = "tests/resources/obscure2.hvp";
pub const OBSCURE2_WII_HVP: &str = "tests/resources/obscure2_wii.hvp";
pub const FINAL_EXAM_HVP: &str = "tests/resources/final_exam_pc.hvp";

pub struct EmptyProgress;

impl RebuildProgress for EmptyProgress {
    fn inc(&self, _: Option<String>) {}
    fn inc_n(&self, _: usize, _: Option<String>) {}
}
