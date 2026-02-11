pub mod edit_file;
pub mod glob;
pub mod grep;
pub mod list_dir;
pub mod memory;
pub mod read_file;
pub mod shell;
pub mod todo;
pub mod web_fetch;
pub mod web_search;
pub mod write_file;

use std::sync::Arc;

use crate::config::Config;
use crate::tools::base::Tool;

use self::edit_file::EditTool;
use self::glob::GlobTool;
use self::grep::GrepTool;
use self::list_dir::ListDirTool;
use self::memory::MemoryTool;
use self::read_file::ReadFileTool;
use self::shell::ShellTool;
use self::todo::TodosTool;
use self::web_fetch::WebFetchTool;
use self::web_search::WebSearchTool;
use self::write_file::WriteFileTool;

pub fn get_all_builtin_tools(config: &Config) -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(ReadFileTool::new(config.clone())),
        Arc::new(WriteFileTool::new(config.clone())),
        Arc::new(EditTool::new(config.clone())),
        Arc::new(ShellTool::new(config.clone())),
        Arc::new(ListDirTool::new(config.clone())),
        Arc::new(GrepTool::new(config.clone())),
        Arc::new(GlobTool::new(config.clone())),
        Arc::new(WebSearchTool::new(config.clone())),
        Arc::new(WebFetchTool::new(config.clone())),
        Arc::new(TodosTool::new(config.clone())),
        Arc::new(MemoryTool::new(config.clone())),
    ]
}
