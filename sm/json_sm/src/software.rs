use serde::{Deserialize, Serialize};

pub type SoftwareType = String;
pub type SoftwareName = String;
pub type SoftwareVersion = String;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct SoftwareModule {
    #[serde(default)]
    pub module_type: Option<SoftwareType>,
    pub name: SoftwareName,
    pub version: Option<SoftwareVersion>,
    pub url: Option<String>,
}

impl SoftwareModule {
    pub fn default_type() -> SoftwareType {
        "default".to_string()
    }

    pub fn is_default_type(module_type: &str) -> bool {
        module_type.is_empty() || module_type == "default"
    }

    pub fn new(
        module_type: Option<SoftwareType>,
        name: SoftwareName,
        version: Option<SoftwareVersion>,
        url: Option<String>,
    ) -> SoftwareModule {
        let mut module = SoftwareModule {
            module_type,
            name,
            version,
            url,
        };
        module.normalize();
        module
    }

    pub fn normalize(&mut self) {
        match &self.module_type {
            Some(module_type) if SoftwareModule::is_default_type(&module_type) => {
                self.module_type = None
            }
            _ => {}
        };

        match &self.version {
            Some(version) if version.is_empty() => self.version = None,
            _ => {}
        };
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub enum SoftwareModuleUpdate {
    Install { module: SoftwareModule },
    Remove { module: SoftwareModule },
}

impl SoftwareModuleUpdate {
    pub fn install(module: SoftwareModule) -> SoftwareModuleUpdate {
        SoftwareModuleUpdate::Install { module }
    }

    pub fn remove(module: SoftwareModule) -> SoftwareModuleUpdate {
        SoftwareModuleUpdate::Remove { module }
    }

    pub fn module(&self) -> &SoftwareModule {
        match self {
            SoftwareModuleUpdate::Install { module } | SoftwareModuleUpdate::Remove { module } => {
                module
            }
        }
    }

    pub fn normalize(&mut self) {
        let module = match self {
            SoftwareModuleUpdate::Install { module } | SoftwareModuleUpdate::Remove { module } => {
                module
            }
        };
        module.normalize();
    }
}
