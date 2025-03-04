use crate::software::{SoftwareModule, SoftwareName, SoftwareType, SoftwareVersion};

use serde::{Deserialize, Serialize};

#[derive(thiserror::Error, Debug, Clone, Deserialize, Serialize, PartialEq)]
pub enum SoftwareError {
    #[error("Failed to finalize updates for {software_type:?}")]
    Finalize {
        software_type: SoftwareType,
        reason: String,
    },

    #[error("Failed to install {module:?}")]
    Install {
        module: SoftwareModule,
        reason: String,
    },

    #[error("Failed to list modules for {software_type:?}")]
    ListError {
        software_type: SoftwareType,
        reason: String,
    },

    #[error("JSON parse error: {reason:?}")]
    ParseError { reason: String },

    #[error("Plugin error for {software_type:?}, reason: {reason:?}")]
    Plugin {
        software_type: SoftwareType,
        reason: String,
    },

    #[error("Failed to prepare updates for {software_type:?}")]
    Prepare {
        software_type: SoftwareType,
        reason: String,
    },

    #[error("Failed to uninstall {module:?}")]
    Remove {
        module: SoftwareModule,
        reason: String,
    },

    #[error("Unknown {software_type:?} module: {name:?}")]
    UnknownModule {
        software_type: SoftwareType,
        name: SoftwareName,
    },

    #[error("Unknown software type: {software_type:?}")]
    UnknownSoftwareType { software_type: SoftwareType },

    #[error("Unexpected module type: {actual:?}, should be: {expected:?}")]
    WrongModuleType {
        actual: SoftwareType,
        expected: SoftwareType,
    },

    #[error("Unknown {software_type:?} version: {name:?} - {version:?}")]
    UnknownVersion {
        software_type: SoftwareType,
        name: SoftwareName,
        version: SoftwareVersion,
    },
}

impl From<serde_json::Error> for SoftwareError {
    fn from(err: serde_json::Error) -> Self {
        SoftwareError::ParseError {
            reason: format!("{}", err),
        }
    }
}
