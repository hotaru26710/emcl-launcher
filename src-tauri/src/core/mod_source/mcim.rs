pub fn modrinth_url(path: &str) -> String {
    format!(
        "{}/{}",
        super::MCIM_MODRINTH_BASE,
        path.trim_start_matches('/')
    )
}

pub fn curseforge_url(path: &str) -> String {
    format!(
        "{}/{}",
        super::MCIM_CURSEFORGE_BASE,
        path.trim_start_matches('/')
    )
}
