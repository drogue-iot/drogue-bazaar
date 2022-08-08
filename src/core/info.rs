/// Project information. Intended as information over all your project's components.
pub struct ProjectInformation {
    /// Project name
    pub name: &'static str,
    /// Version
    pub version: &'static str,
    /// Banner
    pub banner: &'static str,
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
/// It is intended to be present once in a central module of your project, possibly in combination
/// with [`runtime!`].
#[macro_export]
macro_rules! project {
    ($name:literal) => {
        $crate::project!(PROJECT: $name);
    };
    ($v:ident: $name:literal) => {
        $crate::project!($v: $name => r#"______ ______  _____  _____  _   _  _____   _____         _____ 
|  _  \| ___ \|  _  ||  __ \| | | ||  ___| |_   _|       |_   _|
| | | || |_/ /| | | || |  \/| | | || |__     | |    ___    | |  
| | | ||    / | | | || | __ | | | ||  __|    | |   / _ \   | |  
| |/ / | |\ \ \ \_/ /| |_\ \| |_| || |___   _| |_ | (_) |  | |  
|___/  \_| \_| \___/  \____/ \___/ \____/   \___/  \___/   \_/  
"# );
    };
    ($v:ident: $name:expr => $banner:expr) => {
        pub const $v: $crate::core::info::ProjectInformation =
            $crate::core::info::ProjectInformation {
                name: $name,
                version: env!("CARGO_PKG_VERSION"),
                banner: $banner,
        };
    };
}

/// Create a new component information constant.
///
/// This will define a new constant, extracting the name of the component from the cargo file. It is
/// intended to be directly used by [`runtime!`].
#[macro_export]
macro_rules! component {
    ($project:expr) => {
        $crate::core::info::ComponentInformation {
            project: &$project,
            name: env!("CARGO_PKG_NAME"),
            version: env!("CARGO_PKG_VERSION"),
            description: env!("CARGO_PKG_DESCRIPTION"),
        }
    };
    ($v:ident, $project:expr) => {
        pub const $v: $crate::core::info::ComponentInformation = $crate::component!($project);
    };
}
