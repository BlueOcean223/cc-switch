//! 所有权层：CC Switch 只拥有客户端配置里的关键字段，其余字节不碰。
//!
//! - `floor`：每个应用的关键字段和供应商独有字段（纯函数谓词和常量清单）；
//! - `project`：纯函数投影器，供应商行 → 关键字段和独有字段的目标值；
//! - `patch`：按格式保序改写（JSON、TOML、`.env`），解析不了就停；
//! - `residue`：兼容期残留清理（冻结的键值对清单）；
//! - `legacy_routing`：上游 CC Switch 本地路由留下的配置（检测和上游是否还在监听）；
//! - `engine`：写锁、计划、临时文件、首写备份、重读比对。

pub mod engine;
pub mod floor;
pub mod legacy_routing;
pub mod patch;
pub mod project;
pub mod residue;
