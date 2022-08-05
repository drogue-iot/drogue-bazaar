/// Project information. Intended as information over all your project's components.
pub struct ProjectInformation {
    /// Project name
    pub name: &'static str,
    /// Version
    pub version: &'static str,
}

/// Component information. Intended as information over all your project's components.
pub struct ComponentInformation {
    /// Project
    pub project: &'static ProjectInformation,
    /// Component name
    pub name: &'static str,
    /// Version
    pub version: &'static str,
    /// Description
    pub description: &'static str,
}

/// Create a new project information constant.
///
/// This will define a new constant, including the name of the project as well as the version from
/// the cargo file.
///
/// It is intended to be present once in a central module of your project.
#[macro_export]
macro_rules! project {
    ($name:literal) => {
        $crate::project!(PROJECT: $name);
    };
    ($v:ident: $name:literal) => {
        pub const $v: $crate::info::ProjectInformation = $crate::info::ProjectInformation {
            name: $name,
            version: env!("CARGO_PKG_VERSION"),
        };
    };
}

/// Create a new component information constant.
///
/// This will define a new constant, extracting the name of the component from the cargo file. It
/// is intended to be directly used by the [`app!`] macro.
#[macro_export]
macro_rules! component {
    ($project:expr) => {
        $crate::component!(COMPONENT, $project);
    };
    ($v:ident, $project:expr) => {
        pub const $v: $crate::info::ComponentInformation = $crate::info::ComponentInformation {
            project: $project,
            name: env!("CARGO_PKG_NAME"),
            version: env!("CARGO_PKG_VERSION"),
            description: env!("CARGO_PKG_DESCRIPTION"),
        };
    };
}
