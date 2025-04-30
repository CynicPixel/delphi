//parser/parsers.rs
use std::path::Path;

pub fn extract_bench_name<P: AsRef<Path>>(path: P) -> String {
    let path_str = path.as_ref()
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown");
    
    let bench_name = if let Some(pos) = path_str.rfind('.') {
        &path_str[..pos]
    } else {
        path_str
    };
    
    bench_name.to_string()
}