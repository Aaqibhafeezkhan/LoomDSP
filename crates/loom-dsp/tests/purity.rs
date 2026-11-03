//! The crate depends on nothing that reaches its users (ADR-0035). Dev
//! dependencies are allowed, because they only build its tests.

/// True for a TOML key path, such as `target.'cfg(unix)'.dependencies.foo`, that
/// gives users a dependency.
fn is_user_dependency(path: &str) -> bool {
    path.split('.')
        .any(|part| part == "dependencies" || part == "build-dependencies")
}

/// The key paths in `manifest` that would give users a dependency.
fn user_dependencies(manifest: &str) -> Vec<String> {
    let mut section = String::new();
    let mut found = Vec::new();
    for line in manifest.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(rest) = line.strip_prefix('[') {
            // Up to the bracket, so a comment after a header is left out.
            section = rest.split(']').next().unwrap_or(rest).trim().to_string();
        } else {
            let key = line.split('=').next().unwrap_or(line).trim();
            let path = if section.is_empty() {
                key.to_string()
            } else {
                format!("{section}.{key}")
            };
            if is_user_dependency(&path) {
                found.push(path);
            }
        }
    }
    found
}

/// The manifest lists no normal or build dependency, on any target.
#[test]
fn the_crate_has_no_user_dependencies() {
    assert_eq!(user_dependencies(include_str!("../Cargo.toml")), Vec::<String>::new());
}

/// The check above can fail: it sees each way to write a dependency, and
/// allows dev dependencies and the word in other text.
#[test]
fn the_check_sees_each_form() {
    let foo = ["dependencies.foo"];
    assert_eq!(user_dependencies("[dependencies]\nfoo = \"1\"\n"), foo);
    assert_eq!(user_dependencies("[dependencies] # a comment\nfoo = \"1\"\n"), foo);
    assert_eq!(user_dependencies("[ dependencies ]\nfoo = \"1\"\n"), foo);
    assert_eq!(user_dependencies("dependencies.foo = \"1\"\n"), foo);
    assert_eq!(
        user_dependencies("[dependencies.foo]\nversion = \"1\"\n"),
        ["dependencies.foo.version"]
    );
    assert_eq!(
        user_dependencies("[build-dependencies]\nfoo = \"1\"\n"),
        ["build-dependencies.foo"]
    );
    assert_eq!(
        user_dependencies("[target.'cfg(unix)'.dependencies]\nfoo = \"1\"\n"),
        ["target.'cfg(unix)'.dependencies.foo"]
    );
    assert!(user_dependencies("[dev-dependencies]\nfoo = \"1\"\n").is_empty());
    assert!(user_dependencies("[dev-dependencies.foo]\nversion = \"1\"\n").is_empty());
    assert!(user_dependencies("[package]\ndescription = \"no dependencies.\"\n").is_empty());
}
