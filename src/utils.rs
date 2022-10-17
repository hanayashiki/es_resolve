use std::path::PathBuf;

pub fn match_exports_pattern(pattern: &str, target: &str) -> bool {
    let star_index = pattern.find('*');

    match star_index {
        Some(i) => target.starts_with(&pattern[0..i]) && target.ends_with(&pattern[i + 1..]),
        None => pattern == target,
    }
}

pub fn extract_exports_pattern<'a>(pattern: &str, target: &'a str) -> &'a str {
    let star_index = pattern.find('*');

    match star_index {
        Some(i) => &target[i..target.len() - (pattern.len() - i) + 1],
        None => target,
    }
}

pub fn pattern_key_compare(a: &str, b: &str) -> isize {
    let a_pattern_index = a.find('*').unwrap_or(usize::MAX);
    let b_pattern_index = b.find('*').unwrap_or(usize::MAX);

    let base_len_a = if a_pattern_index == usize::MAX {
        a.len()
    } else {
        a_pattern_index + 1
    };

    let base_len_b = if b_pattern_index == usize::MAX {
        b.len()
    } else {
        b_pattern_index + 1
    };

    if base_len_a > base_len_b {
        -1
    } else if base_len_b > base_len_a {
        1
    } else if a_pattern_index == usize::MAX {
        1
    } else if b_pattern_index == usize::MAX {
        -1
    } else if a.len() > b.len() {
        -1
    } else if b.len() > a.len() {
        1
    } else {
        0
    }
}

pub fn add_extension(
    path: &PathBuf,
    extension: impl AsRef<std::path::Path>,
) -> PathBuf {
    match path.extension() {
        Some(ext) => {
            let mut p = PathBuf::from(path);
            let mut ext = ext.to_os_string();
            ext.push(".");
            ext.push(extension.as_ref());
            p.set_extension(ext);
            return p;
        }
        None => path.with_extension(extension.as_ref()),
    }
}
