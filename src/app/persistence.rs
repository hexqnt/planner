use crate::model::Document;

#[cfg(not(target_arch = "wasm32"))]
mod native;

const DOCUMENT_KEY: &str = "planner.document.v1";
const RECOVERY_KEY: &str = "planner.document.recovery.v1";

pub(super) struct LoadedDocument {
    pub document: Document,
    pub persistence: Persistence,
    pub error: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SaveOutcome {
    Unchanged,
    Written,
}

#[derive(Default)]
pub(super) struct Persistence {
    dirty: bool,
    recovery: Option<Recovery>,
    #[cfg(not(target_arch = "wasm32"))]
    native: Option<native::Files>,
}

impl Persistence {
    pub fn load(storage: Option<&dyn eframe::Storage>) -> LoadedDocument {
        Self::load_json(
            storage.and_then(|storage| storage.get_string(DOCUMENT_KEY)),
            storage.and_then(|storage| storage.get_string(RECOVERY_KEY)),
        )
    }

    fn load_json(json: Option<String>, recovery: Option<String>) -> LoadedDocument {
        let mut persistence = Self {
            recovery: recovery.map(|json| Recovery {
                json,
                persisted: true,
            }),
            ..Self::default()
        };
        let Some(json) = json else {
            return LoadedDocument {
                document: Document::default(),
                persistence,
                error: None,
            };
        };
        let parsed = serde_json::from_str::<serde_json::Value>(&json).and_then(|value| {
            let migrated = value
                .get("events")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|events| {
                    events
                        .iter()
                        .any(|event| event.pointer("/identity/uid").is_none())
                });
            serde_json::from_value::<Document>(value).map(|document| (document, migrated))
        });
        match parsed {
            Ok((document, migrated)) => {
                persistence.dirty = migrated;
                LoadedDocument {
                    document,
                    persistence,
                    error: None,
                }
            }
            Err(error) => {
                persistence.recovery = Some(Recovery {
                    json,
                    persisted: false,
                });
                LoadedDocument {
                    document: Document::default(),
                    persistence,
                    error: Some(error.to_string()),
                }
            }
        }
    }

    pub const fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub const fn mark_changed(&mut self) {
        self.dirty = true;
    }

    pub fn recovery_json(&self) -> Option<&str> {
        self.recovery
            .as_ref()
            .map(|recovery| recovery.json.as_str())
    }

    /// Исходный JSON сохраняется до замены документа; без изменений запись не выполняется.
    pub fn save(
        &mut self,
        document: &Document,
        storage: &mut dyn eframe::Storage,
    ) -> Result<SaveOutcome, std::io::Error> {
        if !self.dirty {
            return Ok(SaveOutcome::Unchanged);
        }
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(files) = &mut self.native {
            let json = serde_json::to_vec_pretty(document)?;
            files.save(&json, self.recovery.as_ref())?;
            if let Some(recovery) = &mut self.recovery {
                recovery.persisted = true;
            }
            self.dirty = false;
            return Ok(SaveOutcome::Written);
        }
        let json = serde_json::to_string(document)?;
        if let Some(recovery) = &mut self.recovery
            && !recovery.persisted
        {
            storage.set_string(RECOVERY_KEY, recovery.json.clone());
            recovery.persisted = true;
        }
        storage.set_string(DOCUMENT_KEY, json);
        self.dirty = false;
        Ok(SaveOutcome::Written)
    }
}

struct Recovery {
    json: String,
    persisted: bool,
}

#[cfg(test)]
mod tests;
