use git2::{Cred, FetchOptions, RemoteCallbacks, Repository};
//, Commit, ObjectType, DiffFormat, Oid};

pub fn open_repo(path: &str) -> Repository {
    match Repository::open(path) {
        Ok(repo) => repo,
        Err(e) => panic!("failed to open: {}", e),
    }
}

pub fn clone_repo(url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(|_url, username_from_url, _allowed_types| {
        Cred::ssh_key(
            username_from_url.unwrap_or("git"),
            None, // public key path (optional)
            std::path::Path::new(&format!(
                "{}/.ssh/id_ed25519",
                std::env::var("HOME").unwrap()
            )),
            None, // passphrase
        )
    });

    let mut fetch_options = FetchOptions::new();
    fetch_options.remote_callbacks(callbacks);

    let mut builder = git2::build::RepoBuilder::new();
    builder.fetch_options(fetch_options);

    let repo = builder.clone(url, std::path::Path::new("cloned-repo"))?;

    println!("Repo cloned to: {:?}", repo.path());
    Ok(())
}
