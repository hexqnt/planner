use std::{
    io::{self, Write as _},
    path::{Path, PathBuf},
};

use super::{LoadedDocument, Persistence, Recovery};

pub(super) struct Files {
    directory: Option<PathBuf>,
    load_failed: bool,
}

impl Persistence {
    pub fn load_native() -> LoadedDocument {
        load(eframe::storage_dir("planner"))
    }
}

fn load(directory: Option<PathBuf>) -> LoadedDocument {
    let mut files = Files {
        directory,
        load_failed: false,
    };
    let result = files.read();
    let mut loaded = match result {
        Ok(loaded) => loaded,
        Err(error) => {
            files.load_failed = true;
            let mut loaded = Persistence::load(None);
            loaded.error = Some(format!(
                "{error}. Saving is disabled to protect existing data"
            ));
            loaded
        }
    };
    loaded.persistence.native = Some(files);
    loaded
}

impl Files {
    fn directory(&self) -> io::Result<&Path> {
        self.directory
            .as_deref()
            .ok_or_else(|| io::Error::other("Unable to find application data directory"))
    }

    fn read(&self) -> io::Result<LoadedDocument> {
        let directory = self.directory()?;
        let json = read_optional(&directory.join("document.json"))?;
        let recovery = read_optional(&directory.join("document.recovery.json"))?;
        Ok(Persistence::load_json(json, recovery))
    }

    pub fn save(&self, json: &[u8], recovery: Option<&Recovery>) -> io::Result<()> {
        if self.load_failed {
            return Err(io::Error::other(
                "Saving is disabled after a file read error; restart to retry",
            ));
        }
        let directory = self.directory()?;
        std::fs::create_dir_all(directory)?;
        if let Some(recovery) = recovery
            && !recovery.persisted
        {
            write_atomic(
                directory,
                "document.recovery.json",
                recovery.json.as_bytes(),
            )?;
        }
        write_atomic(directory, "document.json", json)
    }
}

fn read_optional(path: &Path) -> io::Result<Option<String>> {
    match std::fs::read_to_string(path) {
        Ok(json) => Ok(Some(json)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(io::Error::new(
            error.kind(),
            format!("Unable to read {}: {error}", path.display()),
        )),
    }
}

/// Временный файл создаётся в том же каталоге; замена не оставляет частично записанный документ.
fn write_atomic(directory: &Path, name: &str, bytes: &[u8]) -> io::Result<()> {
    let mut temporary = tempfile::NamedTempFile::new_in(directory)?;
    temporary.write_all(bytes)?;
    temporary.as_file().sync_all()?;
    temporary
        .persist(directory.join(name))
        .map_err(|error| error.error)?;
    Ok(())
}

#[cfg(test)]
mod tests;
