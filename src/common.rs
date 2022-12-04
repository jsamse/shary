use std::{
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, Mutex, MutexGuard},
};

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct LocalFile {
    pub path: PathBuf,
    pub name: String,
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct RemoteFile {
    pub addr: SocketAddr,
    pub file: String,
}

#[derive(Clone)]
pub struct AppState {
    mutex: Arc<Mutex<Arc<AppStateData>>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            mutex: Arc::new(Mutex::new(Arc::new(AppStateData {
                local_files: Arc::new(vec![]),
                remote_files: Arc::new(vec![]),
                listeners: Arc::new(vec![]),
            }))),
        }
    }
}

impl AppState {
    pub fn data(&self) -> Arc<AppStateData> {
        let lock = self
            .mutex
            .lock()
            .expect("failed to acquire AppState mutex lock");
        Arc::clone(&lock)
    }

    pub fn add_listener<F>(&self, listener: F)
    where
        F: Fn(&AppStateData) -> () + 'static + Sync + Send,
    {
        let mut lock = self
            .mutex
            .lock()
            .expect("failed to acquire AppState mutex lock");
        let mut listeners = lock.listeners.to_vec();
        listeners.push(Arc::new(listener));
        let data = AppStateData {
            local_files: Arc::clone(&lock.local_files),
            remote_files: Arc::clone(&lock.remote_files),
            listeners: Arc::new(listeners),
        };
        *lock = Arc::new(data);
        AppState::notify_listeners(&lock);
    }

    pub fn add_local_file(&self, path: PathBuf) -> bool {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| Some(String::from(name)));
        let name = match name {
            Some(name) => name,
            None => return false,
        };
        let mut lock = self
            .mutex
            .lock()
            .expect("failed to acquire AppState mutex lock");
        if lock.local_files.iter().any(|f| f.name == name) {
            return false;
        }
        let mut local_files = lock.local_files.to_vec();
        local_files.push(LocalFile { path, name });
        let data = AppStateData {
            local_files: Arc::new(local_files),
            remote_files: Arc::clone(&lock.remote_files),
            listeners: Arc::clone(&lock.listeners),
        };
        *lock = Arc::new(data);
        AppState::notify_listeners(&lock);
        true
    }

    pub fn remove_local_file(&self, name: &str) {
        let mut lock = self
            .mutex
            .lock()
            .expect("failed to acquire AppState mutex lock");
        let mut local_files = lock.local_files.to_vec();
        local_files.retain(|f| f.name != name);
        let data = AppStateData {
            local_files: Arc::new(local_files),
            remote_files: Arc::clone(&lock.remote_files),
            listeners: Arc::clone(&lock.listeners),
        };
        *lock = Arc::new(data);
        AppState::notify_listeners(&lock);
    }

    pub fn set_remote_files(&self, remote_files: Arc<Vec<RemoteFile>>) {
        let mut lock = self
            .mutex
            .lock()
            .expect("failed to acquire AppState mutex lock");
        let data = AppStateData {
            local_files: Arc::clone(&lock.local_files),
            remote_files,
            listeners: Arc::clone(&lock.listeners),
        };
        *lock = Arc::new(data);
        AppState::notify_listeners(&lock);
    }

    fn notify_listeners(lock: &MutexGuard<Arc<AppStateData>>) {
        for listener in lock.listeners.iter() {
            listener(&*lock);
        }
    }
}

pub struct AppStateData {
    pub local_files: Arc<Vec<LocalFile>>,
    pub remote_files: Arc<Vec<RemoteFile>>,
    listeners: Arc<Vec<Arc<dyn Fn(&AppStateData) -> () + Send + Sync>>>,
}
