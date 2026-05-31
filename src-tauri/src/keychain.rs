use keyring::Entry;

const SERVICE: &str = "dock-flare";

pub fn store_token(project_id: &str, token: &str) -> Result<(), String> {
    Entry::new(SERVICE, &format!("token-{}", project_id))
        .map_err(|e| e.to_string())?
        .set_password(token)
        .map_err(|e| e.to_string())
}

pub fn load_token(project_id: &str) -> Option<String> {
    Entry::new(SERVICE, &format!("token-{}", project_id)).ok()?
        .get_password().ok()
}

pub fn delete_token(project_id: &str) {
    if let Ok(entry) = Entry::new(SERVICE, &format!("token-{}", project_id)) {
        let _ = entry.delete_credential();
    }
}
