//! web2doc 库根：聚合各模块（lib + bin 结构，便于单元测试与复用）。
//!
//! 模块按 plan v1.0 / constitution §2 逐里程碑加入。

pub mod assets;
pub mod cli;
pub mod config;
pub mod convert;
pub mod discover;
pub mod error;
pub mod extract;
pub mod fetcher;
pub mod obs;
pub mod pipeline;
pub mod report;
pub mod rewrite;
pub mod rules;
pub mod urlx;
pub mod writer;
