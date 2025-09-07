#![cfg(feature = "cli")]
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::collections::HashMap;
use notify::{Watcher, RecursiveMode, Event, EventKind, Config};
use anyhow::{Result, Context};
pub type ChangeCallback = Box<dyn Fn(&Path) -> Result<()> + Send + Sync>;
pub struct HelixWatcher {
    watcher: notify::RecommendedWatcher,
    callbacks: Arc<Mutex<HashMap<PathBuf, Vec<ChangeCallback>>>>,
    compile_on_change: bool,
    debounce_duration: Duration,
}
impl HelixWatcher {
    pub fn new() -> Result<Self> {
        let callbacks = Arc::new(Mutex::new(HashMap::new()));
        let callbacks_clone = callbacks.clone();
        let watcher = notify::recommended_watcher(move |
            res: Result<Event, notify::Error>|
        {
            if let Ok(event) = res {
                Self::handle_event(event, &callbacks_clone);
            }
        })?;
        Ok(Self {
            watcher,
            callbacks,
            compile_on_change: true,
            debounce_duration: Duration::from_millis(500),
        })
    }
    pub fn with_config(_config: Config) -> Result<Self> {
        let callbacks = Arc::new(Mutex::new(HashMap::new()));
        let callbacks_clone = callbacks.clone();
        let watcher = notify::recommended_watcher(move |
            res: Result<Event, notify::Error>|
        {
            if let Ok(event) = res {
                Self::handle_event(event, &callbacks_clone);
            }
        })?;
        Ok(Self {
            watcher,
            callbacks,
            compile_on_change: true,
            debounce_duration: Duration::from_millis(500),
        })
    }
    pub fn watch_file<P: AsRef<Path>>(
        &mut self,
        path: P,
        callback: ChangeCallback,
    ) -> Result<()> {
        let path = path.as_ref().to_path_buf();
        let mut callbacks = self.callbacks.lock().unwrap();
        callbacks.entry(path.clone()).or_insert_with(Vec::new).push(callback);
        self.watcher
            .watch(&path, RecursiveMode::NonRecursive)
            .context("Failed to watch file")?;
        Ok(())
    }
    pub fn watch_directory<P: AsRef<Path>>(
        &mut self,
        path: P,
        callback: ChangeCallback,
    ) -> Result<()> {
        let path = path.as_ref().to_path_buf();
        let mut callbacks = self.callbacks.lock().unwrap();
        callbacks.entry(path.clone()).or_insert_with(Vec::new).push(callback);
        self.watcher
            .watch(&path, RecursiveMode::Recursive)
            .context("Failed to watch directory")?;
        Ok(())
    }
    pub fn unwatch<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let path = path.as_ref();
        let mut callbacks = self.callbacks.lock().unwrap();
        callbacks.remove(path);
        self.watcher.unwatch(path).context("Failed to unwatch path")?;
        Ok(())
    }
    fn handle_event(
        event: Event,
        callbacks: &Arc<Mutex<HashMap<PathBuf, Vec<ChangeCallback>>>>,
    ) {
        match event.kind {
            EventKind::Modify(_) | EventKind::Create(_) => {
                for path in event.paths {
                    if path.extension().and_then(|s| s.to_str()) != Some("mso") {
                        continue;
                    }
                    let callbacks = callbacks.lock().unwrap();
                    if let Some(cbs) = callbacks.get(&path) {
                        for callback in cbs {
                            if let Err(e) = callback(&path) {
                                eprintln!("Callback error for {:?}: {}", path, e);
                            }
                        }
                    }
                    if let Some(parent) = path.parent() {
                        if let Some(cbs) = callbacks.get(parent) {
                            for callback in cbs {
                                if let Err(e) = callback(&path) {
                                    eprintln!("Callback error for {:?}: {}", path, e);
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
    pub fn set_compile_on_change(&mut self, enable: bool) {
        self.compile_on_change = enable;
    }
    pub fn set_debounce(&mut self, duration: Duration) {
        self.debounce_duration = duration;
    }
}
pub struct CompileWatcher {
    watcher: HelixWatcher,
    compiler: crate::compiler::Compiler,
    output_dir: Option<PathBuf>,
}
impl CompileWatcher {
    pub fn new(optimization_level: crate::compiler::OptimizationLevel) -> Result<Self> {
        Ok(Self {
            watcher: HelixWatcher::new()?,
            compiler: crate::compiler::Compiler::new(optimization_level),
            output_dir: None,
        })
    }
    pub fn output_dir<P: AsRef<Path>>(mut self, dir: P) -> Self {
        self.output_dir = Some(dir.as_ref().to_path_buf());
        self
    }
    pub fn watch<P: AsRef<Path>>(&mut self, dir: P) -> Result<()> {
        let dir = dir.as_ref().to_path_buf();
        let compiler = self.compiler.clone();
        let output_dir = self.output_dir.clone();
        println!("👀 Watching directory: {}", dir.display());
        println!("   Press Ctrl+C to stop");
        self.watcher
            .watch_directory(
                dir,
                Box::new(move |path| {
                    println!("📝 File changed: {}", path.display());
                    match compiler.compile_file(path) {
                        Ok(binary) => {
                            let output_path = if let Some(ref out_dir) = output_dir {
                                let file_name = path
                                    .file_stem()
                                    .unwrap_or_default()
                                    .to_string_lossy();
                                out_dir.join(format!("{}.hlxb", file_name))
                            } else {
                                let mut p = path.to_path_buf();
                                p.set_extension("msob");
                                p
                            };
                            let serializer = crate::compiler::BinarySerializer::new(
                                true,
                            );
                            if let Err(e) = serializer
                                .write_to_file(&binary, &output_path)
                            {
                                eprintln!("   ❌ Failed to write binary: {}", e);
                            } else {
                                println!("   ✅ Compiled to: {}", output_path.display());
                            }
                        }
                        Err(e) => {
                            eprintln!("   ❌ Compilation failed: {}", e);
                        }
                    }
                    Ok(())
                }),
            )?;
        Ok(())
    }
    pub fn run(self) -> Result<()> {
        loop {
            std::thread::sleep(Duration::from_secs(1));
        }
    }
}
pub struct HotReloadManager {
    configs: Arc<Mutex<HashMap<PathBuf, crate::types::HelixConfig>>>,
    watcher: HelixWatcher,
    callbacks: Vec<Box<dyn Fn(&Path, &crate::types::HelixConfig) + Send + Sync>>,
}
impl HotReloadManager {
    pub fn new() -> Result<Self> {
        Ok(Self {
            configs: Arc::new(Mutex::new(HashMap::new())),
            watcher: HelixWatcher::new()?,
            callbacks: Vec::new(),
        })
    }
    pub fn add_config<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let path = path.as_ref().to_path_buf();
        let config = crate::load_file(&path)
            .map_err(|e| anyhow::anyhow!("Failed to load config: {}", e))?;
        {
            let mut configs = self.configs.lock().unwrap();
            configs.insert(path.clone(), config);
        }
        let configs_clone = self.configs.clone();
        let path_clone = path.clone();
        self.watcher
            .watch_file(
                path,
                Box::new(move |changed_path| {
                    match crate::load_file(changed_path) {
                        Ok(new_config) => {
                            let mut configs = configs_clone.lock().unwrap();
                            configs.insert(path_clone.clone(), new_config);
                            println!("🔄 Reloaded config: {}", changed_path.display());
                        }
                        Err(e) => {
                            eprintln!("❌ Failed to reload config: {}", e);
                        }
                    }
                    Ok(())
                }),
            )?;
        Ok(())
    }
    pub fn get_config<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Option<crate::types::HelixConfig> {
        let configs = self.configs.lock().unwrap();
        configs.get(path.as_ref()).cloned()
    }
    pub fn on_change<F>(&mut self, callback: F)
    where
        F: Fn(&Path, &crate::types::HelixConfig) + Send + Sync + 'static,
    {
        self.callbacks.push(Box::new(callback));
    }
    /// Get all managed configurations
    pub fn get_all_configs(&self) -> HashMap<PathBuf, crate::types::HelixConfig> {
        let configs = self.configs.lock().unwrap();
        configs.clone()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;
    #[test]
    fn test_watcher_creation() {
        let watcher = HelixWatcher::new();
        assert!(watcher.is_ok());
    }
    #[test]
    fn test_compile_watcher() {
        let watcher = CompileWatcher::new(crate::compiler::OptimizationLevel::Two);
        assert!(watcher.is_ok());
    }
    #[test]
    fn test_hot_reload_manager() {
        let manager = HotReloadManager::new();
        assert!(manager.is_ok());
    }
    #[test]
    fn test_watch_file() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("test.hlx");
        fs::write(&file_path, "agent \"test\" { model = \"gpt-4\" }")?;
        let mut watcher = HelixWatcher::new()?;
        let called = Arc::new(Mutex::new(false));
        let called_clone = called.clone();
        watcher
            .watch_file(
                &file_path,
                Box::new(move |_path| {
                    let mut c = called_clone.lock().unwrap();
                    *c = true;
                    Ok(())
                }),
            )?;
        fs::write(&file_path, "agent \"test\" { model = \"gpt-4o\" }")?;
        std::thread::sleep(Duration::from_millis(100));
        Ok(())
    }
}